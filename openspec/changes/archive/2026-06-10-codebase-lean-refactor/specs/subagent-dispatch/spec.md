## MODIFIED Requirements

### Requirement: Subagent loop
When dispatched, a subagent SHALL run its own bounded LLM loop via `run_llm_loop` with its own message history, its own tool set (from config), and its own system prompt that includes the ticket context and the task string provided by the main agent. The board client SHALL be built once before the loop begins. The `ContextRepository` SHALL be opened once and passed into `ToolContext`. The subagent loop SHALL NOT have access to `comment`, `done`, or `skip` unless explicitly listed in its `tools` config. The subagent loop SHALL terminate when it calls `return(result)`, returns with no tool calls, or hits its action cap.

#### Scenario: Subagent runs isolated loop
- **WHEN** the main agent calls `dispatch(subagent: "researcher", task: "summarize the linked docs")`
- **THEN** a new LLM loop starts for the researcher subagent with its own history and tool set; the main agent loop is paused until the subagent completes

#### Scenario: Subagent terminates with return
- **WHEN** the subagent calls `return(result: "Here is the summary: ...")`
- **THEN** the subagent loop ends and the result string is returned to the main agent as the tool result of the `dispatch` call

#### Scenario: Subagent hits action cap without returning
- **WHEN** the subagent loop reaches its `max_actions` cap without calling `return`
- **THEN** the dispatch tool returns a synthetic error string to the main agent: "subagent hit action cap without returning a result"

#### Scenario: Subagent loop ends with no tool calls
- **WHEN** the subagent LLM returns a response with no tool calls
- **THEN** the subagent loop ends; the last text response from the LLM is returned as the result

#### Scenario: Board built once per subagent dispatch
- **WHEN** a subagent loop is started
- **THEN** `build_board` is called once before the tool-call loop, not once per iteration
