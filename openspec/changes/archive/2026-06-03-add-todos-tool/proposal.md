## Why

Agents working on multi-step tickets have no built-in way to track progress across tool calls. Without a structured task list, agents lose their place, repeat work, or fail to communicate progress clearly. A `todos` tool — modeled after Crush's implementation — gives every agent (main and subagent) a persistent, per-scope checklist backed by `MemoryStore`.

## What Changes

- New `todos` tool added to `src/agent/tools.rs` with `pending`/`in_progress`/`completed` states and replace-all semantics
- `ToolContext` gains an `agent_scope` field (e.g., `"main"` or the subagent name) to namespace each agent's todos in `MemoryStore`
- `todos` is always available to all agents — not opt-in via config — injected automatically alongside other always-on tools
- `VALID_TOOLS` in `config.rs` updated to include `"todos"` for subagent tool declarations
- Tool dispatch in `dispatch()` routes `"todos"` to its handler
- Main agent loop and subagent loop both receive `todos` in their tool sets unconditionally

## Capabilities

### New Capabilities

- `agent-todos`: Per-scope, persisted task list tool for agents; each agent (main or subagent) maintains an independent list stored in `MemoryStore` under a scoped key

### Modified Capabilities

- `agent-loop`: Main agent and subagent contexts now include `agent_scope` in `ToolContext`; `todos` injected into tool sets unconditionally

## Impact

- `src/agent/tools.rs`: new `dispatch_todos()` handler, new `ToolContext.agent_scope` field, `todos` added to `all_tool_definitions()`
- `src/agent/mod.rs`: `ToolContext` construction updated to pass `agent_scope`; `todos` injected into both main and subagent tool sets
- `src/config.rs`: `"todos"` added to `VALID_TOOLS`
- No new dependencies — uses existing `MemoryStore` and `serde_json`
