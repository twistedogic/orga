## MODIFIED Requirements

### Requirement: Tool-call interface
The LLM client SHALL use the provider's native tool-calling API (not free-form text parsing). The client SHALL send tool definitions, receive `tool_use` responses, execute the tool, and return `tool_result` messages in the next turn, until the cycle ends.

#### Scenario: Tool call round-trip
- **WHEN** the LLM returns a `tool_use` block
- **THEN** the tool is dispatched, the result is collected, and a `tool_result` message is added to the conversation history for the next LLM turn

#### Scenario: LLM stops without tool call
- **WHEN** the LLM returns `stop_reason = end_turn` with no tool use blocks
- **THEN** the cycle ends cleanly

#### Scenario: Full assistant response must be preserved in history
- **WHEN** the LLM returns an assistant message containing tool calls
- **THEN** the full assistant message (including all tool call content, not just text) MUST be appended to history before the tool results are added
- **NOTE**: Storing only the text portion of the assistant response drops the tool call records. Subsequent `tool_result` messages would then reference IDs the LLM has no record of, causing `400 Bad Request: tool id not found`.

#### Scenario: No extra user message between tool results and next LLM call
- **WHEN** tool results have been appended to history
- **THEN** the next LLM call MUST be made with the history ending in those tool results — no additional user message may be appended between the tool results and the LLM call
- **NOTE**: Frameworks that always append a "prompt" as a trailing user message (e.g., `CompletionRequestBuilder`) must be bypassed on continuation turns. The correct approach is to manage the full history manually (`[system, user, assistant{tool_calls}, user{tool_results}]`) and call the model's completion endpoint directly.

#### Scenario: Tool result ID must reference a tool call from the same LLM context
- **WHEN** a `tool_result` message is added to conversation history
- **THEN** the `tool_call_id` field MUST reference the ID of a tool call that was returned by the **same** LLM conversation
- **NOTE**: When dispatching to a subagent, the subagent's internal tool call IDs are from a separate LLM loop and are not valid in the dispatcher's history.
