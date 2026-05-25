## ADDED Requirements

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

#### Scenario: Tool call round-trip
- **WHEN** the LLM returns a `tool_use` block
- **THEN** the tool is dispatched, the result is collected, and a `tool_result` message is added to the conversation history for the next LLM turn

#### Scenario: LLM stops without tool call
- **WHEN** the LLM returns `stop_reason = end_turn` with no tool use blocks
- **THEN** the cycle ends cleanly

### Requirement: rig-core integration
The LLM client SHALL use the `rig-core` crate for provider interaction. All rig-specific code SHALL be isolated within `src/agent/`.

#### Scenario: rig client constructed from config
- **WHEN** a valid `[llm]` config section exists
- **THEN** a rig provider client is constructed with the configured api_key, model, and optional endpoint
