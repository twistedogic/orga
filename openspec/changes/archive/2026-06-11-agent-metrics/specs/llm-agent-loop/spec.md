## MODIFIED Requirements

### Requirement: Run LLM tool-call loop
The `run_llm_loop` function SHALL encapsulate the completion-request / tool-dispatch / history-append cycle used by all agent loops. It SHALL take the `CompletionModel`, a mutable history, a tool set, a `max_steps` cap, a dispatch closure, and an `Arc<AgentMetrics>` recorder along with `model`, `provider`, and `agent` label strings. It SHALL record LLM request, error, duration, and token metrics for every `model.completion(req)` call.

#### Scenario: LLM request observed on success
- **WHEN** `run_llm_loop` completes a successful `model.completion(req).await`
- **THEN** `orga_llm_requests_total{model=<...>,provider=<...>,agent=<...>,kind="ok"}` is incremented, `orga_llm_request_duration_seconds{...}` is observed, and the token counters are incremented from `response.usage`

#### Scenario: LLM error classified and observed
- **WHEN** `run_llm_loop` catches a `CompletionError` from `model.completion(req).await`
- **THEN** the error is classified into `LlmErrorKind`, `orga_llm_requests_total{...kind=<...>}` is incremented with the kind label, and the function returns `Err(OrgaError::LlmError { kind, message })`

#### Scenario: Token usage recorded
- **WHEN** a successful response carries `usage.input_tokens = 100, output_tokens = 50, cached_input_tokens = 20, reasoning_tokens = 10, total_tokens = 180`
- **THEN** the recorder's `input`, `output`, `cached`, `reasoning`, and `total` series for the same labels each gain the reported amount (with `cached` gaining `20`)
