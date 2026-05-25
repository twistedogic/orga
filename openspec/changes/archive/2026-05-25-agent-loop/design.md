## Context

orga is currently a stateless CLI invoked on-demand by an external LLM skill (e.g. a Crush skill wrapping `orga ticket list`, `orga ticket show`, etc.). All scheduling, polling, and LLM interaction happen outside the binary. This change adds a native agent loop mode: `orga agent` embeds a `rig`-based LLM client, polls for assigned tickets, builds context, runs a tool-call cycle, and executes the results — without any external orchestrator.

The existing board, memory, artifact, and compaction APIs are mature and reused unchanged. The loop is a new orchestration layer on top.

## Goals / Non-Goals

**Goals:**
- `orga agent` daemon mode: poll → act → sleep loop
- `orga agent --once`: process current queue then exit (cron/CI-friendly)
- `--dry-run`: log all planned actions without executing board mutations
- LLM tool-call cycle per ticket, bounded by `max_actions_per_ticket`
- `rig-core` as the LLM abstraction; Anthropic and OpenAI-compatible providers supported
- Configurable provider endpoint override (for proxies, local models, etc.)
- `[llm]` config section in existing TOML config
- Full tool set: comment, move, assign, create_sub, checklist, memory, artifact read/write, compact, done (→ return_ticket), skip

**Non-Goals:**
- Multi-turn agentic loops (one bounded cycle per ticket per poll pass)
- Parallel ticket processing (sequential is safer for v1)
- LLM-powered auto-compaction triggered by the loop (compaction tool is available to the LLM, not auto-applied)
- New board backends (loop reuses existing Board trait)

## Decisions

### 1. `rig-core` for LLM interaction

**Decision**: Use `rig-core` as the LLM client library.

**Rationale**: Provides a provider-agnostic interface with first-class tool-calling support for both Anthropic and OpenAI-compatible APIs. Avoids hand-rolling message schemas and tool call/result plumbing. The `rig` `Agent` abstraction maps naturally to the bounded tool-call loop needed here.

**Alternative considered**: Raw `reqwest` calls with hand-written JSON — feasible since reqwest is already present, but significantly more maintenance burden for tool call serialization and provider differences.

### 2. Async runtime (tokio) for the agent loop only

**Decision**: Introduce `tokio` as an optional async runtime, used only within `orga agent`. All existing CLI commands remain synchronous (blocking reqwest, rusqlite).

**Rationale**: `rig-core` is async-first. The agent loop is the only place async is needed. Wrapping the agent entrypoint in `#[tokio::main]` (or a `tokio::runtime::Runtime::block_on`) keeps the blast radius small — no existing code needs to change.

**Alternative considered**: Switching all of orga to async — unnecessary disruption; the board backends use blocking reqwest by design and work well.

### 3. Tool dispatch via enum, not rig's derive macros

**Decision**: Define agent tools as a Rust enum (`AgentTool`) with manual JSON schema definitions passed to rig, rather than using rig's `#[derive(Tool)]` derive macro.

**Rationale**: The tool set maps to existing orga operations (board, memory, artifact, compaction). Tying tool definitions tightly to rig's derive macros would scatter tool logic across many structs. A central enum keeps all dispatch in one `match` in `src/agent/tools.rs` and makes dry-run trivial (check flag before executing the branch).

**Alternative considered**: One struct per tool with `#[derive(Tool)]` — cleaner in isolation but harder to centralize dry-run and logging.

### 4. Artifact context: inline with size cap

**Decision**: Include artifact content inline in LLM context (same as memory), but skip content above a configurable byte cap (`max_artifact_inline_bytes`, default 8192). Above the cap, include metadata only (name, size, committed_at); the LLM can call `get_artifact(name)` to pull full content.

**Rationale**: Small artifacts (notes, summaries) should just appear without requiring an extra tool call. Large artifacts (reports, generated files) would bloat every prompt unnecessarily.

### 5. `done(comment?)` maps to `return_ticket`

**Decision**: The LLM signals completion by calling `done(comment: Option<String>)`, which executes `board.return_ticket(id, comment.as_deref())`.

**Rationale**: Returning the ticket to the creator is the correct handoff signal in the board workflow. An optional comment lets the agent summarize what was done (e.g., "work complete, see artifact report.md").

### 6. Config: `[llm]` section, endpoint override supported

**Decision**:
```toml
[llm]
provider = "anthropic"          # "anthropic" | "openai"
api_key = "sk-ant-..."
model = "claude-opus-4-5"
endpoint = "https://..."        # optional; overrides provider default
poll_interval_secs = 60         # default 60
max_actions_per_ticket = 10     # default 10
max_artifact_inline_bytes = 8192 # default 8192
```

**Rationale**: Endpoint override enables proxies, local OpenAI-compatible models (e.g. Ollama), and corporate gateways without changing provider logic.

### 7. Module layout: `src/agent/`

```
src/agent/
  mod.rs       — AgentLoop struct, run_once(), run_daemon()
  context.rs   — build_ticket_context() → system + user messages
  tools.rs     — AgentTool enum, tool schema definitions, dispatch
  config.rs    — LlmConfig, provider/client construction
```

Keeps agent logic isolated; existing modules (board, memory, artifact) are imported as dependencies, not modified.

## Risks / Trade-offs

- **rig-core API stability** → `rig-core` is pre-1.0; tool-calling API may change. Mitigation: pin to a specific version; isolate all rig usage inside `src/agent/`.
- **Async tokio surface** → Introduces `tokio` to a previously synchronous codebase. Mitigation: use `tokio::runtime::Runtime::block_on` in the CLI dispatch path; the rest of orga stays sync.
- **LLM cost at scale** → Each poll cycle sends a full ticket context. Mitigation: `max_actions_per_ticket` cap; compaction already exists to reduce comment context; `skip()` lets the LLM opt out cheaply.
- **Tool call errors** → If the LLM calls a tool with invalid args (bad ticket ID, missing required field), the error is fed back as a `tool_result` and the cycle continues within the cap. Malformed JSON from rig is surfaced as an `OrgaError` and the ticket is skipped for that cycle.
- **`--dry-run` fidelity** → Read tools (`get_artifact`) execute for real even in dry-run (needed to build accurate context for subsequent decisions). Only mutating tools are suppressed. This is the correct tradeoff.

## Open Questions

- Should the loop log per-ticket actions to the orga log file (already exists via `Logger`), or to stdout? Recommendation: log file for daemon mode, stdout for `--once`.
- Should `max_artifact_inline_bytes` be per-artifact or total across all artifacts in a ticket? Recommendation: per-artifact for simplicity.
