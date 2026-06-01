# llm-client Specification

## Purpose
Provides a provider-agnostic LLM client used by the agent loop. Supports Anthropic and OpenAI-compatible providers with configurable endpoint override. Built on `rig-core`.

## Requirements
### Requirement: Provider configuration
The LLM client SHALL support `anthropic` and `openai` as provider values. The `api_key` and `model` SHALL be required. An optional `endpoint` field SHALL override the provider's default base URL, enabling proxies, local models, and OpenAI-compatible endpoints.

#### Scenario: Anthropic provider with default endpoint
- **WHEN** `provider = "anthropic"` is configured without `endpoint`
- **THEN** the client sends requests to Anthropic's default API base URL

#### Scenario: OpenAI provider with default endpoint
- **WHEN** `provider = "openai"` is configured without `endpoint`
- **THEN** the client sends requests to OpenAI's default API base URL

#### Scenario: Custom endpoint override
- **WHEN** `endpoint = "https://proxy.example.com/v1"` is set
- **THEN** the client sends requests to that URL regardless of provider

#### Scenario: Unknown provider fails at startup
- **WHEN** `provider = "unsupported"` is configured
- **THEN** the CLI exits with a config error listing supported providers

### Requirement: Tool-call interface
The LLM client SHALL use the provider's native tool-calling API (not free-form text parsing). The client SHALL send tool definitions, receive `tool_use` responses, execute the tool, and return `tool_result` messages in the next turn, until the cycle ends.

> **Note (history correctness)**: Three invariants must hold for multi-turn tool-call loops:
> 1. The full assistant response (including all tool call content, not just text) MUST be appended to history. Storing only the text portion drops tool call records, causing subsequent `tool_result` messages to reference unknown IDs.
> 2. After appending tool results, no additional user message may appear before the next LLM call. Frameworks that always append a prompt as a trailing user message must be bypassed on continuation turns — manage the full history manually and call the model completion endpoint directly.
> 3. `tool_result` IDs must reference tool calls from the same LLM conversation. Subagent internal tool call IDs are not valid in the main agent's history.

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

### Requirement: rig-core integration
The LLM client SHALL use the `rig-core` crate for provider interaction. All rig-specific code SHALL be isolated within `src/agent/`.

#### Scenario: rig client constructed from config
- **WHEN** a valid `[llm]` config section exists
- **THEN** a rig provider client is constructed with the configured api_key, model, and optional endpoint
