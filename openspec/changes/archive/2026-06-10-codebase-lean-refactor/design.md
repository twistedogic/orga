## Context

`agent/mod.rs` (797 lines) contains four distinct LLM agent loops — `process_ticket`, `run_subagent_loop`, `run_sleep_time_agent`, `run_defrag_agent` — that share identical `CompletionRequest` construction and tool-dispatch mechanics but cannot be composed due to being inlined. `memory.rs` (926 lines) bundles three unrelated storage backends under one roof. `config.rs` (883 lines) owns markdown-agent loading logic that belongs in the agent layer. Several correctness issues compound these: the board client and context repository are re-initialized on every loop iteration, a declared tool (`move_ticket`) has no dispatch implementation, and dead code silently does nothing.

## Goals / Non-Goals

**Goals:**
- Eliminate ~300 lines of duplicated LLM loop boilerplate via a generic `run_llm_loop` helper
- Fix board and context-repo re-initialization inside hot loops (perf + correctness)
- Split `memory.rs` into `memory/context_repo.rs`, `memory/compaction.rs`, `memory/todo.rs`
- Move markdown agent loader from `config.rs` to `agent/agents.rs`
- Remove `tool_definitions()` alias, dead `old_status` code, and `move_ticket` from `VALID_TOOLS`
- Extract display/output functions from `main.rs` into `src/output.rs`
- All existing tests pass; no behavior changes visible to callers

**Non-Goals:**
- Implementing `move_ticket` functionality (removal only, not addition)
- Changing any CLI interface, config format, or output format
- Performance optimization beyond removing redundant initializations
- Adding new tests (existing coverage is sufficient for pure refactors)

## Decisions

### `run_llm_loop` as a generic async function taking a dispatch closure

The four loops differ only in: (a) which tools are available, (b) what counts as a terminal tool call, (c) the dispatch function body. The shared skeleton is: build request → await completion → extract tool calls → push assistant message → for each call dispatch → push tool result → check terminal. A `run_llm_loop<F>` generic over an async dispatch closure captures this with zero overhead:

```rust
async fn run_llm_loop<M, F, Fut>(
    model: &M,
    history: &mut Vec<Message>,
    tools: Vec<ToolDefinition>,
    max_steps: usize,
    dispatch: F,
) -> Result<LoopOutcome, OrgaError>
where
    M: CompletionModel,
    F: Fn(String, String) -> Fut,  // (tool_name, args) -> result
    Fut: Future<Output = (String, bool)>,  // (result, is_terminal)
```

**Alternative considered**: trait object `Box<dyn Fn(...) -> BoxFuture<...>>` — rejected because it requires boxing every dispatch call and adds indirection without benefit since all call sites are monomorphized anyway.

**Alternative considered**: an enum over the four loop types — rejected because it would need to be extended for every future agent variant, coupling loop mechanics to agent semantics.

### `CompletionRequest` builder or default

All four loops construct `CompletionRequest` with identical `None` fields and only vary `chat_history` and `tools`. Rather than a full builder pattern, a single `make_completion_request(history, tools)` free function that fills in all `None` fields is sufficient — it's only 10 lines and avoids an additional type.

### Memory module split: flat re-exports from `memory/mod.rs`

Callers currently use `crate::memory::{CompactionStore, ContextRepository, TodoStore}`. After the split, `memory/mod.rs` re-exports all three public types so no import paths change anywhere in the codebase:

```rust
// src/memory/mod.rs
pub use compaction::CompactionStore;
pub use context_repo::ContextRepository;
pub use todo::TodoStore;
```

**Alternative considered**: rename the module paths and update all imports — rejected because it touches every file that imports from `memory` with no benefit.

### Markdown agent loader moves to `agent/agents.rs`

`load_markdown_agents` and `parse_markdown_agent` are called from `AppConfig::load`, which means `config.rs` currently holds agent-domain parsing logic. The move: create `src/agent/agents.rs` with the two functions as `pub(crate)`, import from `config.rs`. The call site in `AppConfig::load` does not change.

### `move_ticket` removal from `VALID_TOOLS`

No dispatch branch exists and no subagent currently uses it. Removing it from `VALID_TOOLS` makes config validation reject misconfigured subagents that reference it instead of silently returning an error at runtime. When the tool is eventually implemented it will be re-added.

## Risks / Trade-offs

- **`run_llm_loop` closure captures** — async closures that capture `Arc` references need careful lifetime annotation; risk is a compile error, not a runtime issue. Mitigation: compile-test incrementally, starting with the simplest loop (`run_sleep_time_agent`) before tackling `process_ticket`.
- **Memory module split** — the re-export approach means `rustdoc` shows items in `memory` rather than their submodule; acceptable trade-off since this is an internal crate.
- **`move_ticket` removal** — any user who has `tools = ["move_ticket"]` in a subagent config will get a startup validation error after upgrade. Risk is low (feature was never documented as working), and the error message is clear.

## Migration Plan

Pure internal refactor — no data migrations, no config changes. Rollout:
1. Split `memory.rs` (isolated, all tests travel with the code)
2. Move markdown agent loader
3. Add `run_llm_loop` helper and wire `run_sleep_time_agent` first (smallest consumer)
4. Wire `run_defrag_agent`, `run_subagent_loop`, `process_ticket` in sequence
5. Fix board/repo re-init, remove dead code, delete alias
6. Extract `output.rs`
7. `cargo test` green throughout

Rollback: revert is a straightforward `git revert` since no external state changes.

## Open Questions

- None — all decisions are resolved. Implementation can proceed directly from tasks.
