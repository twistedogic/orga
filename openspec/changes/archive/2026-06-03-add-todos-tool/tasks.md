## 1. ToolContext

- [x] 1.1 Add `agent_scope: String` field to `ToolContext` in `src/agent/tools.rs`
- [x] 1.2 Set `agent_scope: "main".to_string()` at the main agent `ToolContext` construction site in `src/agent/mod.rs`
- [x] 1.3 Set `agent_scope: sub_cfg.name.clone()` at the subagent `ToolContext` construction site in `src/agent/mod.rs`

## 2. Todos tool implementation

- [x] 2.1 Define `TodoItem` and `TodosArgs` structs in `src/agent/tools.rs`
- [x] 2.2 Implement `dispatch_todos()` handler: load from `MemoryStore`, diff transitions, validate statuses, store updated list, return summary string
- [x] 2.3 Add scope key helper: sanitize `agent_scope` (replace non-alphanumeric with `_`), format as `__todos_<scope>__`
- [x] 2.4 Add `todos` to `dispatch()` match arm in `src/agent/tools.rs`
- [x] 2.5 Add `todos` tool definition to `all_tool_definitions()` in `src/agent/tools.rs`

## 3. Tool injection

- [x] 3.1 In main agent tool setup (`src/agent/mod.rs`), push `"todos"` unconditionally to both the subagent-configured and non-subagent-configured tool name lists
- [x] 3.2 In subagent loop tool setup (`src/agent/mod.rs`), push `"todos"` alongside `"return"` unconditionally

## 4. Config validation

- [x] 4.1 Add `"todos"` to `VALID_TOOLS` in `src/config.rs` so explicit subagent config declarations are accepted

## 5. Verification

- [x] 5.1 Run `cargo build` — confirm clean
- [x] 5.2 Run `cargo test` — confirm all tests pass
- [x] 5.3 Add unit tests for `dispatch_todos`: first call (empty baseline), status transition tracking, invalid status rejection, scope key sanitization
