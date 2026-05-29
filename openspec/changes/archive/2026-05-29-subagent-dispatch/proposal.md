## Why

The current agent loop treats every ticket the same way — one flat LLM loop with the same tools and skills. This forces the agent to be a generalist, limiting how focused and capable it can be for specific task types. A subagent dispatch model allows specialized agents (defined in config) to handle work with the right tools and skills for the job, while the main agent focuses on communication and coordination.

## What Changes

- The main agent loop becomes a **dispatcher**: a short loop whose job is to classify the ticket, delegate work to a subagent, and communicate results to the user via comments.
- A new `dispatch(subagent, task)` tool is added to the main agent's tool set.
- A new `return(result)` tool is added for subagents to report results back to the main agent.
- **BREAKING**: The main agent no longer has access to the full tool set by default — its tools are narrowed to `comment`, `dispatch`, `skip`, `done`.
- Subagents are defined in config (`[[subagents]]` blocks) with a name, description, tool set, optional skill list, and optional model override.
- The main agent selects a subagent by matching the ticket against subagent descriptions (LLM reasoning, not keyword matching).
- The subagent runs its own bounded LLM loop and returns a result string via `return(result)`.
- The main agent is conversational across board cycles — it can ask clarifying questions before dispatching, and re-dispatch on follow-up requests.

## Capabilities

### New Capabilities

- `subagent-dispatch`: Config-driven subagent registry, main-agent dispatch tool, subagent loop with `return` tool, result propagation back to main agent.

### Modified Capabilities

- `agent-loop`: Main agent loop is restructured as a dispatcher; per-ticket cycle behavior changes significantly.
- `agent-tools`: Main agent tool set narrowed; `dispatch` tool added; `return` tool added for subagents.
- `config`: New `[[subagents]]` config section with name, description, tools, skills, model, max_actions.

## Impact

- `src/agent/mod.rs` — `process_ticket` restructured; subagent loop extracted
- `src/agent/tools.rs` — `dispatch` tool added; tool set becomes configurable per agent type
- `src/agent/context.rs` — subagent context builder added (different system prompt)
- `src/config.rs` — `SubagentConfig` struct and `AppConfig.subagents` field added
- No new external dependencies required
