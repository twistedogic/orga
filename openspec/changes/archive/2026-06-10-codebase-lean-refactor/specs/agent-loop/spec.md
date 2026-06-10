## MODIFIED Requirements

### Requirement: Per-ticket LLM tool-call cycle
For each selected ticket, the loop SHALL build a context (system prompt + ticket content) and run a bounded tool-call cycle using `run_llm_loop`. The board client SHALL be built once before the loop begins and reused across all iterations. The `ContextRepository` SHALL be opened once per ticket and passed into `ToolContext`. The system prompt SHALL include an "## Available Skills" section listing all discovered skills when a skills folder is configured, and an "## Active Skills" section with full skill bodies for any skills that match the ticket. The cycle SHALL stop when the LLM calls `done()`, `skip()`, returns with no tool calls, or the `max_actions_per_ticket` cap is reached. When subagents are configured, the main agent cycle SHALL use a narrowed tool set (`comment`, `dispatch`, `skip`, `done`, `set_memory`, `compact`, `todos`) and the system prompt SHALL include the names and descriptions of all configured subagents. When no subagents are configured, the existing flat loop behavior SHALL apply unchanged, with `todos` added to the tool set. `ToolContext` SHALL carry an `agent_scope` field identifying the current agent: `"main"` for the main agent, and the subagent name for subagents.

#### Scenario: ToolContext carries agent scope for main agent
- **WHEN** the main agent loop constructs `ToolContext`
- **THEN** `agent_scope` is set to `"main"`

#### Scenario: ToolContext carries agent scope for subagent
- **WHEN** the subagent loop constructs `ToolContext`
- **THEN** `agent_scope` is set to the subagent's name as defined in config

#### Scenario: Main agent cycle with subagents configured
- **WHEN** subagents are configured and the main agent processes a ticket
- **THEN** the main agent receives only the narrowed tool set

#### Scenario: Board built once per ticket cycle
- **WHEN** the main agent processes a ticket
- **THEN** `build_board` is called once before the tool-call loop, not once per iteration

#### Scenario: ContextRepository opened once per ticket cycle
- **WHEN** the main agent processes a ticket
- **THEN** `ContextRepository::open` is called once before the tool-call loop
