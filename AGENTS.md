# AGENTS.md

Guidelines for AI agents and contributors working on this project.

## Project

`orga` is a Rust CLI (edition 2024) that lets an LLM agent act as a first-class member of a Trello or Linear kanban board. It runs in two modes:

- **CLI mode** — read/mutate commands invoked by an agent skill (`skills/orga/SKILL.md` covers the session script: `whoami`, `ticket list`, `ticket show`, `comment`, `assign`, `create_sub`, `return`, `compact`).
- **Agent mode** — `orga agent [--once]` is a long-lived (or one-shot) loop that polls the board, dispatches tickets through an LLM, and posts results back. `--once` runs a single pass without binding the metrics server.

### Tech stack

- Language: Rust, edition 2024
- CLI: `clap` v4 (derive)
- HTTP: `reqwest` (blocking, rustls) — board APIs and (via `rig-core`) LLM provider APIs
- LLM: `rig-core` 0.37 (providers behind the `CompletionModel` trait)
- Storage: `rusqlite` (bundled) for per-ticket memory, `TodoStore`, and `CompactionStore`
- VCS: `git2` for the git-backed long-term memory
- Config: TOML via `serde` + `toml`
- Async: `tokio` (`rt-multi-thread`, `macros`, `time`, `process`)
- Metrics: `prometheus` 0.13 served over a hand-rolled one-shot TCP listener (no `axum`, no `/healthz`, no keep-alive)
- Errors: `thiserror` via `OrgaError`; LLM failures classified by `LlmErrorKind` (`network | rate_limit | auth | parse | backend | other`) through a single `From<CompletionError>` impl

### Architecture (single source of truth)

- `Board` trait + `build_board` factory; backends are `TrelloBackend` and `LinearBackend`. All CLI commands and the agent loop dispatch through it.
- `AppConfig` is TOML from `~/.orga/config.toml` (overrides: `--config`, `ORGA_CONFIG`). One agent identity per config, one board per config. Sections: `[agent]`, `[board]`, `[trello]` / `[linear]`, `[memory]`, `[llm]`, `[logging]`, `[metrics]`, `[skills]`, `[workspace]`, plus top-level `comment_compaction_threshold`, `[[workflow]]`, `[[subagents]]`. Markdown subagents in `agents/` next to the config are merged into `subagents` at load time.
- `run_agent(once, dry_run, &AppConfig, Arc<Logger>)` in `src/agent/mod.rs` is the agent-mode entry point. It builds a `RunContext { config, logger, metrics, dry_run, llm_cfg }` and dispatches to one-shot or daemon loop.
- `run_llm_loop` in `src/agent/loop_runner.rs` is the **single LLM call site** for the main, subagent, sleep-time, and defrag loops. All four call it.
- `MAIN_TOOLS: &[&str]` in `src/agent/tools.rs` is the **single source of truth** for the main agent's tools — it drives both `tool_definitions_for(...)` and the "Available tools: …" prose in the system prompt. Adding/renaming a tool is a one-line edit there; the prompt and the code cannot drift.
- System prompts live as `src/agent/prompts/{main_agent,dispatcher,sleep_time,defrag}.md` and are loaded via `include_str!` with `{placeholder}` substitution. The workspace `AGENTS.md` is appended to the main agent's context when present.
- `ContextRepository` (`src/memory/context_repo.rs`) is git-backed long-term memory with `list`, `read`, `write` (commits on every write), `search`, `delete`, and a defrag pass (`analyze`). `CompactionStore` and `TodoStore` are SQLite. Legacy `MemoryStore` is kept for per-ticket notes.
- `AgentMetrics` (`src/metrics.rs`) records LLM request count/errors/duration, token usage, tool calls by `scope × outcome`, and ticket-processing duration. Exposed only in daemon mode when `[metrics]` is set. The bind is non-fatal — a failure logs a warning and continues without metrics.
- LLM error classification lives in `src/error.rs` (`classify_completion_error` + `From<CompletionError>`). History inspection is typed (`Message::User`/`Assistant`, `UserContent::ToolResult`, `AssistantContent::ToolCall`) — no `serde_json::to_string(...)` substring sniffing.

### Module layout

- `src/lib.rs` — module exports
- `src/main.rs` — clap tree: `init {board, agent}`, `ticket {list, show, comment, assign, create_sub, return, compact, decompact}`, `memory {list, read, write, search, defrag, delete}`, `columns`, `whoami`, `systemd install`, `agent`
- `src/error.rs` — `OrgaError`, `LlmErrorKind`, classification, `From<CompletionError>`
- `src/config.rs` — `AppConfig`, `validate`, every `*Config` struct
- `src/output.rs` — human and JSON output formatting
- `src/logging.rs` — structured `Logger`
- `src/models.rs` — `Ticket`, `TicketSummary`, `Comment`, `CommentCompaction`, `Member`, `Column`
- `src/board/{mod,trello,linear,agent_tag}.rs`
- `src/memory/{mod,context_repo,compaction,todo}.rs` (legacy `src/memory.rs` for per-ticket notes)
- `src/agent/{mod,agents,config,context,loop_runner,skills,tools}.rs` + `src/agent/prompts/*.md`
- `src/metrics.rs` — Prometheus recorder + hand-rolled one-shot TCP listener
- `src/init.rs` — interactive setup wizards
- `src/systemd.rs` — unit file generator/installer
- `src/workspace.rs` — workspace path resolution

### Key constraints

- Agent tool permissions: comment, assign, create sub-tickets, return (never close). `done{}` calls `board.return_ticket()` and triggers the sleep-time agent.
- All read commands support `--json`; errors with `--json` produce `{"error": "<message>"}` on stderr with non-zero exit.
- Public APIs return `Result<_, OrgaError>`. No `unwrap()` / `expect()` in production paths.
- Relative paths are logical identifiers (memory keys, git index paths, workspace listings shown to the language model), so every `Path` → `String` conversion of a relative path goes through `workspace::to_slash` and always renders `/`. Never use `to_string_lossy()` on a relative path — it emits `\` on Windows and breaks path identity comparisons (e.g. `ContextRepository::delete`).
- Metrics listener is a one-shot TCP server: `HTTP/1.0 200 OK`, `Content-Type: text/plain; version=0.0.4`, `Connection: close`. The previous axum-based server (and `/healthz`) was removed in the `2026-07-02-simplify-agent-loop` refactor — do not reintroduce axum.
- New board backends implement the `Board` trait and register in `build_board`. No `axum`, no `tower`, no `hyper`.

## openspec workflow

`openspec/` is the source of truth for design decisions and capability contracts:

- `openspec/specs/<capability>/spec.md` — current requirement for a capability. Edit these in place for additive changes (a feature, a behavior tightening). Never rewrite history.
- `openspec/changes/archive/<YYYY-MM-DD>-<name>/` — proposal, design, tasks, and spec deltas for a change. The latest is `2026-07-02-simplify-agent-loop`. New work follows the same shape:
  - `proposal.md` — **Why** and **What Changes** (under 500 words), plus a list of `### New Capabilities` and `### Modified Capabilities` with one-line `> Added` / `> Modified` requirement blocks when the change adds or alters a requirement.
  - `design.md` — non-obvious decisions, trade-offs, alternatives rejected.
  - `tasks.md` — checkboxed implementation steps. Phrased so each box is one verifiable action.
  - `specs/<capability>/spec.md` — requirement deltas (use `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements`).
- `openspec/config.yaml` — schema (`spec-driven`) and shared project context (the `context:` block). Update it when the project shape changes (new major dependency, new module, breaking constraint); keep it consistent with this file.

Spec wording is normative: scenarios in `#### Scenario: …` blocks are the contract tests must satisfy. When you change behavior, change the spec in the same change.

## Documentation

- Never use acronyms without explaining them in documents. The first time an acronym appears, spell out the full term followed by the acronym in parentheses (e.g., "Application Programming Interface (API)"). Subsequent uses may use the acronym alone.

## Commit Messages

- When writing a commit message, never add your agent name as author or co-author. Commits must reflect the human contributor as author and must not include agent names in the author or co-author trailers.

## Bug Fixes

- When doing a bug fix, always start by reproducing the bug and add a failing test case before changing production code. The failing test must demonstrate the bug, and the fix must turn it green. Never merge a bug fix without a regression test.

## Technical Decisions

- When making a technical decision, do not give much weight to development cost and time. Instead, prefer correctness, readability, simplicity, and long-term maintainability. Short-term effort is a secondary concern; the chosen approach should be one we are willing to live with for years.

## Observability

- Always consider observability of the application in development.
- Prefer structured logging (key/value fields, consistent log levels, machine-parseable format) over unstructured log strings.
- For servers, prefer Prometheus metrics (counters, gauges, histograms) exposed on a standard scrape endpoint, in addition to structured logs.

## Maintenance of AGENTS.md

- Keep AGENTS.md up to date on key design decisions and development workflows. When a decision is made or a workflow changes, update this file in the same change so it remains the source of truth for future contributors and agents.
- When touching `openspec/` (adding a spec, archiving a change, revising the `context:` block), make sure the corresponding entry in this file still matches reality — module layout, dependency list, key constraints, and "single source of truth" invariants drift together.

## CI / Automation

- Use [Task](https://taskfile.dev/docs/getting-started) (a `Taskfile.yml`) as the CI runner and shared automation entry point; keep commands reproducible locally and in CI.
