## Context

The agent tool system in `src/agent/tools.rs` defines all tools as `ToolDefinition` values and routes calls through `dispatch()`. `ToolContext` carries per-invocation state. Two construction sites exist: the main agent loop (`mod.rs:237`) and the subagent loop (`mod.rs:442`).

`MemoryStore` is already available at both sites and persists key-value pairs per ticket in SQLite. It's the natural backing store for todos.

Crush's `todos` tool uses replace-all semantics: the agent passes the entire updated list on each call. The tool computes transitions (just_completed, just_started) by diffing old vs new. State is stored per-session in Crush; in orga, it maps to a per-scope MemoryStore key.

## Goals / Non-Goals

**Goals:**
- `todos` tool available unconditionally to all agents (main and subagent)
- Per-scope persistence: main agent and each subagent maintain independent lists
- Todos survive across invocations of the same agent on the same ticket
- Faithful port of Crush semantics: replace-all, three statuses, transition tracking, same response format

**Non-Goals:**
- Cross-ticket or cross-agent visibility of todo lists
- UI rendering of todos (response text is sufficient)
- Enforcing "exactly one in_progress" in code — left to LLM instruction

## Decisions

**Store todos in `MemoryStore` under a scoped key**

Key format: `__todos_<scope>__` where scope is `"main"` for the main agent and the subagent name for subagents (e.g., `__todos_researcher__`).

Stored as a JSON array of `TodoItem`. This reuses existing infrastructure with zero new dependencies.

Alternatives considered:
- Separate SQLite table: overkill for a simple list; MemoryStore already handles arbitrary blobs
- In-memory only: todos wouldn't survive across invocations, breaking the persistence requirement

**Add `agent_scope: String` to `ToolContext`**

The todos handler needs to know which scoped key to read/write. Passing it via `ToolContext` is the cleanest path — the scope is known at construction time in both the main loop and subagent loop.

Main agent construction site sets `agent_scope: "main".to_string()`.  
Subagent construction site sets `agent_scope: sub_cfg.name.clone()`.

Alternatives considered:
- Pass scope as a tool argument: would require the LLM to know its own scope, fragile
- Compute from a global context: no global context exists in this codebase

**`todos` always available — not in `VALID_TOOLS` opt-in list, injected unconditionally**

`todos` has no board side effects and no destructive behavior. Always injecting it (like `skip`/`return`) avoids config noise. In both the main agent tool setup and subagent tool setup, `todos` is pushed alongside other always-present tools.

The subagent loop currently auto-injects `return`; `todos` follows the same pattern.  
The main agent's tool set is built conditionally in `mod.rs` based on subagent config; `todos` is added to both branches.

`VALID_TOOLS` in `config.rs` still gets `"todos"` added so explicit subagent config declarations don't fail validation — but it won't be required.

## Risks / Trade-offs

- [Replace-all race] If the main agent and a subagent both call `todos` on the same ticket in the same run, they write different scoped keys — no collision. Risk: none.
- [Key naming] Subagent names with special characters could produce malformed keys. → Mitigation: sanitize scope name (replace non-alphanumeric with `_`) when building the key, consistent with how `WorkspaceStore` sanitizes ticket IDs.
- [JSON parse failure] If the stored blob is corrupt, treat as empty list and start fresh rather than returning an error — better to lose history than break the tool.

## Migration Plan

No migration needed. `MemoryStore` keys are additive. Existing tickets with no todos simply start with an empty list on first call.
