## MODIFIED Requirements

### Requirement: Per-ticket LLM tool-call cycle
For each selected ticket, the loop SHALL build a context (system prompt + ticket content) and run a bounded tool-call cycle with the LLM. When subagents are configured, the main agent cycle SHALL use a narrowed tool set (`comment`, `dispatch`, `skip`, `done`) and the system prompt SHALL include the names and descriptions of all configured subagents. When no subagents are configured, the existing flat loop behavior SHALL apply unchanged. The cycle SHALL stop when the LLM calls `done()`, `skip()`, returns with no tool calls, or the `max_actions_per_ticket` cap is reached.

#### Scenario: Main agent cycle with subagents configured
- **WHEN** subagents are configured and the main agent processes a ticket
- **THEN** the system prompt includes subagent names and descriptions; the available tools are `comment`, `dispatch`, `skip`, `done`

#### Scenario: Main agent cycle without subagents configured
- **WHEN** no subagents are configured
- **THEN** the agent uses the full tool set and existing behavior unchanged

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
