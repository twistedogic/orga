# llm-agent-loop Specification

## Purpose
TBD

## Requirements

### Requirement: Generic loop helper
The system SHALL provide a `run_llm_loop` function that accepts a completion model, mutable message history, tool definitions, a step cap, and an async dispatch closure. It SHALL run the LLM completion → extract tool calls → dispatch → append history cycle, stopping when: (a) no tool calls are returned, (b) the dispatch closure signals terminal, or (c) the step cap is reached. It SHALL return a `LoopOutcome` indicating how it terminated.

#### Scenario: Loop stops when LLM returns no tool calls
- **WHEN** the LLM completion response contains no tool calls
- **THEN** the loop exits with `LoopOutcome::NoToolCalls`

#### Scenario: Loop stops at step cap
- **WHEN** the number of dispatched tool calls reaches `max_steps`
- **THEN** the loop exits with `LoopOutcome::CapReached`

#### Scenario: Loop stops on terminal signal
- **WHEN** the dispatch closure returns `is_terminal = true` for a tool call
- **THEN** the loop exits with `LoopOutcome::Terminal` after appending that tool result to history

#### Scenario: History is mutated in place
- **WHEN** the loop runs successfully
- **THEN** assistant messages and tool results are appended to the passed `history` Vec

### Requirement: CompletionRequest construction helper
The system SHALL provide a `make_completion_request` free function that constructs a `CompletionRequest` from a history slice and tool list, filling all optional fields with `None`. This SHALL be the single authoritative place for request construction.

#### Scenario: Request constructed from history and tools
- **WHEN** `make_completion_request` is called with non-empty history and a tool list
- **THEN** a valid `CompletionRequest` is returned with `chat_history` set from history and `tools` set from the list; all other fields are `None`

### Requirement: LLM call metrics
The `run_llm_loop` function SHALL observe LLM request volume, duration, and token usage metrics for every `model.completion(req).await` call. It SHALL accept an `Arc<AgentMetrics>` recorder along with `model`, `provider`, and `agent` label strings. On success, it SHALL increment the LLM request counter with `kind="ok"`, observe the call duration, and record the token usage. On error, it SHALL classify the failure into an `LlmErrorKind` (see the `error` capability), increment the LLM request counter with the classified `kind` label, and return `Err(OrgaError::LlmError { kind, message })`. The duration SHALL be observed on both success and error paths.

#### Scenario: LLM request observed on success
- **WHEN** `run_llm_loop` completes a successful `model.completion(req).await`
- **THEN** `orga_llm_requests_total{model=<...>,provider=<...>,agent=<...>,kind="ok"}` is incremented, `orga_llm_request_duration_seconds{...}` is observed, and the token counters are incremented from `response.usage`

#### Scenario: LLM error classified and observed
- **WHEN** `run_llm_loop` catches a `CompletionError` from `model.completion(req).await`
- **THEN** the error is classified into `LlmErrorKind`, `orga_llm_requests_total{...kind=<...>}` is incremented with the kind label, and the function returns `Err(OrgaError::LlmError { kind, message })`

#### Scenario: Token usage recorded
- **WHEN** a successful response carries `usage.input_tokens = 100, output_tokens = 50, cached_input_tokens = 20, reasoning_tokens = 10, total_tokens = 180`
- **THEN** the recorder's `input`, `output`, `cached`, `reasoning`, and `total` series for the same labels each gain the reported amount (with `cached` gaining `20`)

