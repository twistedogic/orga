## Context

The current `process_ticket` function runs a single flat LLM loop with the full tool set for every ticket. The agent is implicitly specialized by column prompts and skill injection, but the loop shape itself never changes. This change restructures the agent into two layers: a main agent (dispatcher/communicator) and subagents (specialized workers), where subagents are defined in config.

The main agent loop already exists (`process_ticket` in `src/agent/mod.rs`). The tool dispatch infrastructure exists (`src/agent/tools.rs`). The context builder exists (`src/agent/context.rs`). This change layers subagent routing on top of the existing machinery without replacing it wholesale.

## Goals / Non-Goals

**Goals:**
- Main agent becomes a short dispatcher loop: classify → dispatch → communicate
- Subagents are config-driven with per-agent tool sets, skills, and optional model override
- Main agent and subagent run separate LLM loops with separate histories
- Subagent result propagates back to main agent as a tool result string
- Main agent retains conversational state across board cycles (via comment thread)
- Backward compatible: if no subagents are configured, behavior falls back to the current flat loop

**Non-Goals:**
- Parallel subagent execution (sequential only in this change)
- Subagents spawning their own subagents (one level of nesting only)
- Streaming or long-running subagent processes
- Subagents posting comments or calling `done` directly

## Decisions

### Decision: Main agent as a pure dispatcher, not a general loop

The main agent's tool set is narrowed to `comment`, `dispatch`, `skip`, `done`. It does not have access to `commit_artifact`, `move_ticket`, etc.

**Rationale**: Giving the main agent the full tool set would blur the boundary between dispatcher and worker. A narrow tool set makes the main agent's role unambiguous and prevents it from accidentally doing work that belongs to a subagent.

**Alternative considered**: Keep the full tool set on the main agent, add `dispatch` as an optional extra. Rejected because the agent would have to decide each turn whether to do the work itself or delegate — this creates inconsistent behavior and harder-to-reason-about prompts.

### Decision: Subagent selection via LLM reasoning over config descriptions

The main agent receives a list of subagent names and descriptions in its system prompt and uses LLM reasoning to select the right one via the `dispatch` tool call.

**Rationale**: Keyword matching would be brittle. A classification pre-call would add latency and a failure point. Embedding the subagent descriptions in the main agent's system prompt and letting it reason over them in the same turn it decides to dispatch is natural and requires no extra LLM calls.

**Alternative considered**: A dedicated classification LLM call before the main loop. Rejected — adds latency, complexity, and a separate failure mode.

### Decision: `dispatch(subagent, task)` as a tool, `return(result)` as subagent terminal

The main agent calls `dispatch` as a tool; the tool handler runs the subagent loop synchronously and returns the result string. The subagent calls `return(result)` to terminate its loop and surface the result.

**Rationale**: Fits naturally into the existing tool dispatch model. The main agent's loop continues after `dispatch` returns, allowing it to comment the result, ask follow-ups, or call `done`. `return` mirrors the role of `done` in the current loop — a terminal tool that ends the cycle and carries a payload.

**Alternative considered**: Subagent loop terminates on no-tool-call; last text response is the result. Rejected — implicit and unreliable. `return(result)` makes the result boundary explicit.

### Decision: Fallback to current loop when no subagents configured

If `[[subagents]]` is absent from config, `process_ticket` runs the existing flat loop with the full tool set. No behavior change for existing deployments.

**Rationale**: Zero-friction migration. Existing users don't need to update their config.

### Decision: Subagent context includes the full ticket + injected task string

The subagent receives the same ticket context as the main agent (built by `build_context`), plus the task string from `dispatch(subagent, task)` injected into the system prompt or as a prefix to the user message.

**Rationale**: The subagent needs full ticket context to do its work. The task string from the main agent focuses it on what specifically to do.

## Risks / Trade-offs

**[Risk] Main agent loop cap is spent on `dispatch` calls** → The `max_actions_per_ticket` cap on the main agent counts `dispatch` as one action (regardless of how many actions the subagent uses internally). Subagent has its own cap from config.

**[Risk] Subagent result string can be arbitrarily large** → Main agent receives it as a tool result and may include it in a comment verbatim. Mitigation: subagent system prompt instructs it to return concise summaries; board comment limits apply naturally.

**[Risk] No subagent matches the ticket** → Main agent should fall back to `comment`-ing the user ("I don't know how to handle this") and `skip`-ing, rather than erroring. This is prompt-level behavior, not a code error.

**[Risk] Subagent loop hits its cap without calling `return`** → The dispatch tool returns a synthetic error string ("subagent hit action cap without returning a result") to the main agent, which can decide how to handle it.

## Open Questions

- Should the main agent have access to `set_memory` and `compact`? Likely yes — memory is ticket-scoped and the main agent is the one with full conversation context. Not a blocker for initial implementation.
- Should subagent tool access be validated at config load time (i.e., reject unknown tool names)? Recommended but not strictly required for v1.
