## Why

The codebase has grown organically to ~7,300 lines across a small file count, producing several fat files with mixed responsibilities, duplicated LLM agent loop boilerplate, and repeated resource initialization inside hot loops. Addressing this now keeps future features easy to add and the codebase easy to navigate.

## What Changes

- **Extract generic LLM agent loop** — the four agent loops in `agent/mod.rs` (`process_ticket`, `run_subagent_loop`, `run_sleep_time_agent`, `run_defrag_agent`) share ~200 lines of identical `CompletionRequest` construction and tool-dispatch boilerplate; consolidate into a reusable `run_llm_loop` helper
- **Fix board rebuilt per-iteration** — `build_board` is called inside the tool-dispatch loop on every iteration; move it before the loop and pass the result into `ToolContext`
- **Fix `ContextRepository` opened repeatedly** — repo is opened 4+ times per ticket across `process_ticket` and its callees; open once and thread through
- **Split `memory.rs` into three modules** — `ContextRepository`, `CompactionStore`, and `TodoStore` are unrelated stores in one 926-line file; split into `memory/context_repo.rs`, `memory/compaction.rs`, `memory/todo.rs`
- **Move markdown agent loader out of `config.rs`** — `load_markdown_agents` + `parse_markdown_agent` are agent-domain logic (~100 lines) living in the config module; move to `agent/agents.rs`
- **Remove `tool_definitions()` alias** — `tool_definitions()` is a one-line wrapper around `all_tool_definitions()`; delete the alias and update the single call site
- **Fix dead code in `dispatch_todos`** — `old_status` HashMap is built then discarded via `let _ = old_status.get(...)` with no effect; remove it
- **Fix `move_ticket` in `VALID_TOOLS` with no dispatch** — `config.rs` lists `move_ticket` as a valid subagent tool but there is no `dispatch_move_ticket`; remove from `VALID_TOOLS` until the tool is implemented
- **Move display functions to `output.rs`** — `print_column_list`, `print_ticket_summary_list`, `print_ticket_detail`, and `exit_error` in `main.rs` are presentation logic; move to a dedicated `output.rs` module

## Capabilities

### New Capabilities

- `llm-agent-loop`: A reusable `run_llm_loop` abstraction that encapsulates the completion-request / tool-dispatch / history-append cycle used by all agent loops

### Modified Capabilities

- `agent-loop`: Implementation refactored to use `run_llm_loop`; board and context repo opened once per ticket rather than per-iteration
- `subagent-dispatch`: `run_subagent_loop` refactored to use `run_llm_loop`
- `sleep-time-agent`: `run_sleep_time_agent` and `run_defrag_agent` refactored to use `run_llm_loop`
- `agent-memory`: `memory.rs` split into three focused modules under `memory/`
- `subagent-markdown-loader`: markdown agent loader moved from `config.rs` to `agent/agents.rs`
- `agent-tools`: `tool_definitions()` alias removed; `move_ticket` removed from `VALID_TOOLS`
- `cli-commands`: display/output functions extracted from `main.rs` into `output.rs`

## Impact

- All code is internal; no public API changes, no config format changes, no CLI behavior changes
- `lib.rs` re-exports may need updating for renamed/split modules
- Tests in `memory.rs` move alongside their modules
