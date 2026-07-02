## MODIFIED Requirements

### Requirement: Per-ticket LLM tool-call cycle
For each selected ticket, the loop SHALL build a context (system prompt + ticket content) and run a bounded tool-call cycle using `run_llm_loop`. The board client SHALL be built once before the loop begins and reused across all iterations. The `ContextRepository` SHALL be opened once per ticket and passed into `ToolContext`. The system prompt SHALL include an "## Available Skills" section listing all discovered skills when a skills folder is configured, and an "## Active Skills" section with full skill bodies for any skills that match the ticket. The cycle SHALL stop when the LLM calls `done()`, `skip()`, returns with no tool calls, or the `max_actions_per_ticket` cap is reached. When subagents are configured, the main agent cycle SHALL use a narrowed tool set (`comment`, `dispatch`, `skip`, `done`, `set_memory`, `compact`, `todos`) and the system prompt SHALL include the names and descriptions of all configured subagents. When no subagents are configured, the existing flat loop behavior SHALL apply unchanged, with `todos` added to the tool set. `ToolContext` SHALL carry an `agent_scope` field identifying the current agent: `"main"` for the main agent, and the subagent name for subagents.

The cycle SHALL observe metrics via the `Arc<AgentMetrics>` threaded from `run_daemon`: the LLM request counter, error counter, duration histogram, and token counters are recorded by `run_llm_loop`; tool call counts and errors are recorded by the dispatch closure with `scope = "main"`.

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

#### Scenario: Main agent tool calls observed with scope main
- **WHEN** the main agent dispatch closure runs a tool
- **THEN** `orga_agent_tool_calls_total{tool=<name>,scope="main",outcome=<ok|error>}` is incremented

### Requirement: Sequential ticket processing
The loop SHALL process tickets one at a time in the order returned by `list_assigned`. Parallel processing SHALL NOT occur in v1.

#### Scenario: Multiple tickets processed in order
- **WHEN** three tickets are waiting on the agent
- **THEN** they are processed sequentially; the second ticket is not started until the first cycle completes

#### Scenario: Per-ticket processing duration observed
- **WHEN** a ticket completes (success, error, skip, or cap-reached)
- **THEN** `orga_agent_ticket_processing_duration_seconds{outcome=<...>}` is observed with the wall-clock elapsed time

### Requirement: Error isolation per ticket
If the LLM cycle for a ticket fails (network error, LLM error, tool dispatch error), the loop SHALL log the error and continue to the next ticket without aborting the run. The `run_llm_loop` SHALL classify LLM errors into an `LlmErrorKind` enum (`network` | `rate_limit` | `auth` | `parse` | `backend` | `other`) at the call site, construct `OrgaError::LlmError { kind, message }`, and increment the LLM error counter with the classified `kind` label.

#### Scenario: LLM error on one ticket does not abort the loop
- **WHEN** the LLM call fails for ticket A
- **THEN** the error is logged, ticket A is skipped for this cycle, and ticket B is processed normally

#### Scenario: LLM error classified into bounded kind
- **WHEN** the LLM call fails with a transport timeout
- **THEN** the returned error has `LlmErrorKind::Network` and the `orga_llm_requests_total{...kind="network"}` series is incremented
