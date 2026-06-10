# subagent-dispatch Specification

## Purpose
Config-driven subagent dispatch system. The main agent acts as a dispatcher/communicator; specialized subagents run isolated LLM loops with task-specific tool sets and skills. Results flow back to the main agent which communicates them to the user.

## Requirements
### Requirement: Subagent registry
The agent SHALL support a list of subagent definitions in config under `[[subagents]]`. Each subagent SHALL have a `name` (unique string), `description` (used by the main agent for routing), and `tools` (list of tool names the subagent may call). Each subagent MAY additionally specify `skills` (list of skill names to inject), `model` (LLM model override), and `max_actions` (per-subagent action cap override).

#### Scenario: Subagent defined in config
- **WHEN** the config contains a `[[subagents]]` block with `name`, `description`, and `tools`
- **THEN** that subagent is available for the main agent to dispatch to

#### Scenario: Subagent with optional overrides
- **WHEN** a `[[subagents]]` block includes `model = "..."` and `max_actions = 20`
- **THEN** the subagent loop uses that model and cap instead of the global LLM config defaults

#### Scenario: No subagents configured
- **WHEN** no `[[subagents]]` blocks exist in config
- **THEN** the agent falls back to the existing flat loop behavior with no behavior change

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

### Requirement: Subagent skill injection
If a subagent config specifies a `skills` list, those skills SHALL be loaded from the configured skills path and injected into the subagent's system prompt, regardless of keyword matching. If `skills` is not specified, standard skill matching by ticket title applies.

#### Scenario: Explicit skills injected into subagent
- **WHEN** a subagent config has `skills = ["writing-style", "templates"]`
- **THEN** those skill bodies are included in the subagent's system prompt

#### Scenario: No skills list falls back to keyword matching
- **WHEN** a subagent config has no `skills` field
- **THEN** skills are matched against the ticket title using the existing match_skills logic

### Requirement: dispatch tool
The main agent SHALL have access to a `dispatch(subagent, task)` tool. `subagent` is the name of a configured subagent. `task` is a string describing what the subagent should do. The tool is synchronous — it blocks until the subagent loop completes and returns the result string.

#### Scenario: Successful dispatch
- **WHEN** the main agent calls `dispatch(subagent: "researcher", task: "find relevant prior art")`
- **THEN** the researcher subagent loop runs and its result is returned as the tool result

#### Scenario: Dispatch to unknown subagent
- **WHEN** the main agent calls `dispatch(subagent: "nonexistent", task: "...")`
- **THEN** an error string is returned: "error: no subagent named 'nonexistent' is configured"

### Requirement: return tool
Subagents SHALL have access to a `return(result)` terminal tool. Calling it ends the subagent loop and surfaces the result string to the main agent. `return` SHALL be treated as a terminal tool (ends the loop immediately).

#### Scenario: Subagent calls return with result
- **WHEN** the subagent calls `return(result: "Analysis complete: ...")`
- **THEN** the subagent loop terminates and the result string is passed back to the main agent
