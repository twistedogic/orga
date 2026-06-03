# agent-loop Specification

## Purpose
Self-driving poll-act loop that processes tickets assigned to the agent. Polls the board for actionable tickets, builds LLM context, runs a bounded tool-call cycle per ticket, and handles errors in isolation.

## Requirements
### Requirement: Agent loop command
The CLI SHALL expose an `orga agent` subcommand that starts the agent loop. It SHALL accept `--once` (process current queue then exit) and `--dry-run` (log planned actions without executing board mutations) flags. Both flags MAY be combined.

#### Scenario: Daemon mode starts
- **WHEN** `orga agent` is invoked without `--once`
- **THEN** the loop polls assigned tickets, processes each, then sleeps for `poll_interval_secs` and repeats until interrupted

#### Scenario: Single-pass mode exits after one cycle
- **WHEN** `orga agent --once` is invoked
- **THEN** the loop processes the current assigned ticket queue once and exits with code 0

#### Scenario: Dry-run suppresses mutations
- **WHEN** `orga agent --dry-run` is invoked
- **THEN** no board mutations (comment, move, assign, artifact write, return_ticket, etc.) are executed; planned actions are logged to stdout

#### Scenario: Dry-run with --once
- **WHEN** `orga agent --once --dry-run` is invoked
- **THEN** the queue is processed once with all mutations suppressed

### Requirement: Ticket selection
The loop SHALL only process tickets where the last commenter is not the agent (i.e., tickets waiting on the agent). Completed tickets SHALL be skipped.

#### Scenario: Tickets awaiting agent are processed
- **WHEN** the loop runs and a ticket has `last_commenter_is_agent = false` and `completed = false`
- **THEN** that ticket enters the LLM tool-call cycle

#### Scenario: Tickets already responded to are skipped
- **WHEN** a ticket has `last_commenter_is_agent = true`
- **THEN** that ticket is skipped for this cycle

### Requirement: User message contains ticket metadata
The user message SHALL include the following ticket metadata fields, in order: title, ID, column, URL, today's date, creator (if present), and assignees (if present).

#### Scenario: User message includes today's date field
- **WHEN** any agent (main or subagent) constructs a user message
- **THEN** the message SHALL include `**Today's date:** YYYY-MM-DD` after the URL field and before the creator field

#### Scenario: User message includes all other metadata
- **WHEN** any agent constructs a user message
- **THEN** the message SHALL still include title, ID, column, URL, creator, and assignees as before

### Requirement: Per-ticket LLM tool-call cycle
For each selected ticket, the loop SHALL build a context (system prompt + ticket content) and run a bounded tool-call cycle with the LLM. The system prompt SHALL include an "## Available Skills" section listing all discovered skills when a skills folder is configured, and an "## Active Skills" section with full skill bodies for any skills that match the ticket. The cycle SHALL stop when the LLM calls `done()`, `skip()`, returns with no tool calls, or the `max_actions_per_ticket` cap is reached. When subagents are configured, the main agent cycle SHALL use a narrowed tool set (`comment`, `dispatch`, `skip`, `done`, `set_memory`, `compact`, `todos`) and the system prompt SHALL include the names and descriptions of all configured subagents. When no subagents are configured, the existing flat loop behavior SHALL apply unchanged, with `todos` added to the tool set. `ToolContext` SHALL carry an `agent_scope` field identifying the current agent: `"main"` for the main agent, and the subagent name for subagents.

#### Scenario: ToolContext carries agent scope for main agent
- **WHEN** the main agent loop constructs `ToolContext`
- **THEN** `agent_scope` is set to `"main"`

#### Scenario: ToolContext carries agent scope for subagent
- **WHEN** the subagent loop constructs `ToolContext`
- **THEN** `agent_scope` is set to the subagent's name as defined in config

#### Scenario: Main agent cycle with subagents configured
- **WHEN** subagents are configured and the main agent processes a ticket
- **THEN** the system prompt includes subagent names and descriptions; the available tools are `comment`, `dispatch`, `skip`, `done`, `set_memory`, `compact`, `todos`

#### Scenario: Main agent cycle without subagents configured
- **WHEN** no subagents are configured
- **THEN** the agent uses the full tool set including `todos` and existing behavior unchanged

#### Scenario: Cycle completes with done()
- **WHEN** the LLM calls the `done` tool during a ticket cycle
- **THEN** `return_ticket` is executed (with the optional comment) and the cycle ends

#### Scenario: Cycle completes with skip()
- **WHEN** the LLM calls the `skip` tool
- **THEN** no mutation is made, the cycle ends, and the ticket remains in the queue for the next poll

#### Scenario: Cycle hits max_actions cap
- **WHEN** the number of tool calls for a ticket reaches `max_actions_per_ticket`
- **THEN** the cycle ends without calling `done` or `skip`; the ticket is left in its current state

#### Scenario: Cycle completes with no tool calls
- **WHEN** the LLM returns a response with no tool calls (stop_reason = end_turn)
- **THEN** the cycle ends; no mutation is made

#### Scenario: Skills injected into system prompt at cycle start
- **WHEN** the LLM cycle starts for a ticket
- **THEN** the system prompt includes available skills listing and any matched active skills before the first LLM call

### Requirement: Sequential ticket processing
The loop SHALL process tickets one at a time in the order returned by `list_assigned`. Parallel processing SHALL NOT occur in v1.

#### Scenario: Multiple tickets processed in order
- **WHEN** three tickets are waiting on the agent
- **THEN** they are processed sequentially; the second ticket is not started until the first cycle completes

### Requirement: Error isolation per ticket
If the LLM cycle for a ticket fails (network error, LLM error, tool dispatch error), the loop SHALL log the error and continue to the next ticket without aborting the run.

#### Scenario: LLM error on one ticket does not abort the loop
- **WHEN** the LLM call fails for ticket A
- **THEN** the error is logged, ticket A is skipped for this cycle, and ticket B is processed normally
