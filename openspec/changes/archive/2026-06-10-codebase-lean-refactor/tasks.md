## 1. Split memory.rs into three modules

- [x] 1.1 Create `src/memory/` directory with `mod.rs` that re-exports `ContextRepository`, `CompactionStore`, and `TodoStore`
- [x] 1.2 Move `ContextRepository`, `ContextEntry`, `RepoStats` and all methods + tests into `src/memory/context_repo.rs`
- [x] 1.3 Move `CompactionStore`, `CompactionRecord` and all methods + tests into `src/memory/compaction.rs`
- [x] 1.4 Move `TodoStore` and all methods into `src/memory/todo.rs`
- [x] 1.5 Delete `src/memory.rs`; update `src/lib.rs` to use `mod memory` pointing at the new directory
- [x] 1.6 Run `cargo test` — all tests green

## 2. Move markdown agent loader out of config.rs

- [x] 2.1 Create `src/agent/agents.rs` with `load_markdown_agents`, `parse_markdown_agent`, and `SubagentFrontmatter` moved from `config.rs`
- [x] 2.2 Export `load_markdown_agents` as `pub(crate)` from `src/agent/agents.rs`; add `pub mod agents;` to `src/agent/mod.rs`
- [x] 2.3 Update `config.rs` to call `crate::agent::agents::load_markdown_agents` instead of the local function; remove the local functions and `SubagentFrontmatter`
- [x] 2.4 Run `cargo test` — all tests green

## 3. Extract output functions from main.rs

- [x] 3.1 Create `src/output.rs` with `print_column_list`, `print_ticket_summary_list`, `print_ticket_detail`, and `exit_error` moved from `main.rs`
- [x] 3.2 Add `mod output;` to `main.rs`; replace all direct calls with `output::*` calls
- [x] 3.3 Run `cargo test` — all tests green

## 4. Clean up dead code and tool registry issues

- [x] 4.1 Remove `tool_definitions()` alias in `agent/tools.rs`; update the single call site in `agent/mod.rs` to call `all_tool_definitions()` directly
- [x] 4.2 Remove `"move_ticket"` from `VALID_TOOLS` in `config.rs`
- [x] 4.3 Remove dead `old_status` HashMap and the `let _ = old_status.get(...)` no-op in `dispatch_todos` in `agent/tools.rs`
- [x] 4.4 Run `cargo test` — all tests green

## 5. Add run_llm_loop helper and make_completion_request

- [x] 5.1 Add `LoopOutcome` enum (`NoToolCalls`, `CapReached`, `Terminal`) to `agent/mod.rs` (or a new `agent/loop_runner.rs`)
- [x] 5.2 Implement `make_completion_request(history: &[Message], tools: Vec<ToolDefinition>) -> CompletionRequest` as a free function
- [x] 5.3 Implement `run_llm_loop<M, F, Fut>(model, history, tools, max_steps, dispatch) -> Result<LoopOutcome, OrgaError>`
- [x] 5.4 Run `cargo test` — all tests green

## 6. Wire run_llm_loop into sleep-time and defrag agents

- [x] 6.1 Refactor `run_sleep_time_agent` to use `run_llm_loop`; open `ContextRepository` once before calling the loop
- [x] 6.2 Refactor `run_defrag_agent` to use `run_llm_loop`; open `ContextRepository` once before calling the loop
- [x] 6.3 Run `cargo test` — all tests green

## 7. Wire run_llm_loop into subagent loop

- [x] 7.1 Refactor `run_subagent_loop` to use `run_llm_loop`; build board and open context repo once before the loop
- [x] 7.2 Run `cargo test` — all tests green

## 8. Wire run_llm_loop into main agent loop

- [x] 8.1 Refactor `process_ticket` to use `run_llm_loop`; build board once before the loop; open `ContextRepository` once and pass into `ToolContext`
- [x] 8.2 Verify `build_board` is no longer called inside any loop body across `agent/mod.rs`
- [x] 8.3 Run `cargo test` — all tests green
- [x] 8.4 Smoke-test with `cargo run -- agent --once --dry-run` against a real config (if available)
