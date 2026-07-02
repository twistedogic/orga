# agent-metrics Specification

## Purpose
Prometheus instrumentation for the orga agent loop. Exposes LLM request volume, error rate, duration, and token usage; tool call volume and error rate; and per-ticket processing duration. A `/metrics` HTTP endpoint serves the data in OpenMetrics text format, bound only in daemon mode and only when the user opts in via a `[metrics]` config section.

## Requirements

### Requirement: Metrics recorder
The system SHALL provide a `pub struct AgentMetrics` in `src/metrics.rs` that owns a `prometheus::Registry` and the following typed metrics:

| Metric | Type | Labels | Purpose |
|--------|------|--------|---------|
| `orga_llm_requests_total` | Counter | `model`, `provider`, `agent`, `kind` | LLM completion calls; `kind` = `ok` for successes, otherwise the `LlmErrorKind` label of the failure |
| `orga_llm_request_duration_seconds` | Histogram | `model`, `provider`, `agent` | Wall-clock duration of LLM completion calls |
| `orga_llm_tokens_total` | Counter | `model`, `provider`, `agent`, `kind` | Token usage, `kind` ∈ {`input`, `output`, `cached`, `reasoning`, `total`} |
| `orga_agent_tool_calls_total` | Counter | `tool`, `scope`, `outcome` | Tool dispatches, `scope` ∈ {`main`, `subagent`, `sleep`}, `outcome` ∈ {`ok`, `error`} |
| `orga_agent_ticket_processing_duration_seconds` | Histogram | `outcome` | Time to process a single ticket, `outcome` ∈ {`success`, `error`, `skipped`, `cap_reached`} |

`AgentMetrics::new()` SHALL construct the recorder with a fresh `Registry` and register all five metrics. The recorder SHALL be `Send + Sync` and intended to be shared via `Arc<AgentMetrics>`. The `kind` label on the LLM requests counter SHALL always be drawn from the `LlmErrorKind` enum defined in `error.rs` (including the literal `ok` for successes). The `kind` label on the tokens counter SHALL always be one of the five fixed values listed above. The `scope` and `outcome` labels on tool calls and tickets SHALL always be drawn from the fixed sets listed above.

#### Scenario: Recorder increments counters
- **WHEN** `record_llm_request("claude-opus-4-7", "anthropic", "main")` is called
- **THEN** `orga_llm_requests_total{model="claude-opus-4-7",provider="anthropic",agent="main",kind="ok"}` is incremented by 1

#### Scenario: Recorder classifies errors
- **WHEN** `record_llm_error("claude-opus-4-7", "anthropic", "main", LlmErrorKind::RateLimited)` is called
- **THEN** `orga_llm_requests_total{model="claude-opus-4-7",provider="anthropic",agent="main",kind="rate_limit"}` is incremented by 1

#### Scenario: Token counter aggregates usage
- **WHEN** `record_tokens(...)` is called with a rig `Usage` of `input_tokens=100, output_tokens=50, cached_input_tokens=20, cache_creation_input_tokens=5, reasoning_tokens=10, total_tokens=185`
- **THEN** the `input` series gains 100, `output` gains 50, `cached` gains 25, `reasoning` gains 10, `total` gains 185

#### Scenario: Tool call outcome
- **WHEN** `record_tool_call("comment", "main", ToolOutcome::Ok)` is called
- **THEN** `orga_agent_tool_calls_total{tool="comment",scope="main",outcome="ok"}` is incremented by 1

#### Scenario: Encode renders text
- **WHEN** `encode()` is called on a recorder with at least one observation
- **THEN** the returned string contains `# HELP` and `# TYPE` lines for every registered metric and a non-zero sample line for the observed series, and is renderable by `promtool check metrics`

### Requirement: LLM error classification
The system SHALL classify failures from `model.completion(req).await` into the `LlmErrorKind` enum at a single classification site. The mapping SHALL be:

| Condition | `LlmErrorKind` |
|-----------|---------------|
| Transport: `is_timeout()` or `is_connect()` | `network` |
| HTTP 429 | `rate_limit` |
| HTTP 401 or 403 | `auth` |
| HTTP 4xx (other) | `parse` |
| HTTP 5xx | `backend` |
| Anything else | `other` |

Classification SHALL inspect the inner `reqwest::Error` (when present) by downcasting from the rig `CompletionError`. If the inner error is not a `reqwest::Error`, the classifier SHALL fall back to `LlmErrorKind::Other`. `run_llm_loop` SHALL construct `OrgaError::LlmError { kind, message }` from the classification and return it from the call site; the public `run_llm_loop` signature SHALL remain `Result<(LoopOutcome, String), OrgaError>`. The `record_llm_error` call SHALL happen before the function returns. The `LlmErrorKind` enum SHALL expose a `pub fn as_str(&self) -> &'static str` returning the same labels currently produced by its `Display` impl; the `Display` impl is removed in favor of `as_str()`.

#### Scenario: Network timeout classified
- **WHEN** the LLM call fails with a `reqwest::Error` whose `is_timeout()` is true
- **THEN** the returned error has `LlmErrorKind::Network`, the `orga_llm_requests_total{...kind="network"}` series is incremented, and the error message preserves the original transport message

#### Scenario: HTTP 429 classified
- **WHEN** the LLM call fails with a `reqwest::Error` whose `is_status()` reports 429
- **THEN** the returned error has `LlmErrorKind::RateLimited`

#### Scenario: Non-reqwest inner error falls back to other
- **WHEN** the LLM call fails with a rig `CompletionError` whose source is not a `reqwest::Error`
- **THEN** the returned error has `LlmErrorKind::Other`

### Requirement: Daemon-only metrics endpoint
The `/metrics` HTTP endpoint SHALL be bound only when **both** of the following are true: (a) the CLI was invoked in daemon mode (`orga agent` without `--once`), and (b) the config contains a `[metrics]` section. If either condition is false, no port SHALL be bound. The endpoint SHALL serve the OpenMetrics text format produced by `AgentMetrics::encode()` with content type `text/plain; version=0.0.4; charset=utf-8`. The implementation SHALL use a one-shot TCP listener: each accepted connection is read until the request headers are drained, the encoded body is written with `Connection: close`, and the connection is closed. Keep-alive is not required and SHALL NOT be implemented. No `/healthz` endpoint SHALL be served; the previous `/healthz` route that returned a static `ok` body is removed. The bind address SHALL come from `[metrics].listen_addr`; the default SHALL be `127.0.0.1:9090`. If the bind fails (port in use, permission denied), the daemon SHALL log a warning via the existing `Logger` and continue without metrics — the daemon SHALL NOT crash.

#### Scenario: Daemon with [metrics] binds port
- **WHEN** `orga agent` is invoked with a config that contains `[metrics] listen_addr = "127.0.0.1:9090"`
- **THEN** a TCP listener is bound on `127.0.0.1:9090` before the first poll cycle and `GET /metrics` returns a 200 with the recorder's text output

#### Scenario: /healthz is not served
- **WHEN** `orga agent` is invoked with a config that contains `[metrics]`
- **THEN** a `GET /healthz` request receives a connection-close with no 200 response (the route does not exist; the connection is closed after the request line is read)

#### Scenario: --once mode does not bind port
- **WHEN** `orga agent --once` is invoked
- **THEN** no TCP listener is bound, regardless of whether `[metrics]` is present in the config

#### Scenario: Missing [metrics] does not bind port
- **WHEN** `orga agent` is invoked in daemon mode and the config has no `[metrics]` section
- **THEN** no TCP listener is bound and the daemon runs unchanged

#### Scenario: Bind failure is non-fatal
- **WHEN** `orga agent` is invoked in daemon mode with `[metrics]` configured but the port is already in use
- **THEN** a `WARN` entry is written to the structured log file describing the bind error, no metrics endpoint is served, and the daemon continues processing tickets

### Requirement: LLM token usage observation
For every successful `model.completion(req).await` in `run_llm_loop`, the function SHALL call `AgentMetrics::record_tokens(...)` with the `response.usage` value. The five `kind` series SHALL be updated as defined in the recorder requirement above. If the provider did not report any token field (all zeros), the corresponding series SHALL still be incremented by zero (no-op) so that the label set is consistent across providers.

#### Scenario: Token usage recorded on success
- **WHEN** `run_llm_loop` completes a successful LLM call that returned a `CompletionResponse` with `usage.input_tokens = 100, output_tokens = 50`
- **THEN** the recorder's `input` and `output` series for the same `(model, provider, agent)` labels are each incremented by the reported values

#### Scenario: Zero-usage provider still records
- **WHEN** a provider returns `Usage::default()` (all zeros)
- **THEN** the recorder is called and the five series are unchanged (no underflow, no error)

### Requirement: Tool call observation
For every tool dispatch in the main agent, subagent, and sleep-time dispatch closures, the recorder SHALL observe the tool name, the scope (`main` | `subagent` | `sleep`), and the outcome (`ok` | `error`). The outcome SHALL be classified by inspecting the dispatch result string: if the string starts with `"error:"`, the outcome is `error`; otherwise `ok`. This matches the existing tool-error contract defined in `agent-tools` spec.

#### Scenario: Main agent tool success observed
- **WHEN** the main agent dispatch closure runs `comment("hi")` and the result is `"comment posted"`
- **THEN** `orga_agent_tool_calls_total{tool="comment",scope="main",outcome="ok"}` is incremented by 1

#### Scenario: Main agent tool error observed
- **WHEN** the main agent dispatch closure runs `comment("hi")` and the result is `"error: network unreachable"`
- **THEN** `orga_agent_tool_calls_total{tool="comment",scope="main",outcome="error"}` is incremented by 1

#### Scenario: Subagent and sleep-time scope labels
- **WHEN** a tool is dispatched inside `run_subagent_loop`, the `scope` label is `subagent`; when dispatched inside `run_sleep_time_agent`'s sleep-time loop, the `scope` label is `sleep`

### Requirement: Ticket processing duration
For every ticket processed in the daemon's poll loop, the recorder SHALL observe the wall-clock elapsed time and the outcome into `orga_agent_ticket_processing_duration_seconds`. The `outcome` label SHALL be:

| Condition | `outcome` label |
|-----------|----------------|
| ticket processing returned `Ok(TicketProcessingOutcome::Success)` | `success` |
| ticket processing returned `Err(_)` | `error` |
| ticket processing returned `Ok(TicketProcessingOutcome::Skipped)` | `skipped` |
| ticket processing returned `Ok(TicketProcessingOutcome::CapReached)` | `cap_reached` |

The histogram buckets SHALL cover 0.1s to 300s.

#### Scenario: Successful ticket observed
- **WHEN** ticket processing returns `Ok(TicketProcessingOutcome::Success)` (e.g., the LLM cycle ended via the `done` tool)
- **THEN** `orga_agent_ticket_processing_duration_seconds{outcome="success"}` is observed with the elapsed duration

#### Scenario: Failed ticket observed
- **WHEN** ticket processing returns `Err(_)` (LLM error or board error)
- **THEN** `orga_agent_ticket_processing_duration_seconds{outcome="error"}` is observed with the elapsed duration

#### Scenario: Cap-reached ticket observed
- **WHEN** ticket processing returns `Ok(TicketProcessingOutcome::CapReached)` (the LLM cycle hit the action cap without `done` or `skip`)
- **THEN** `orga_agent_ticket_processing_duration_seconds{outcome="cap_reached"}` is observed

### Requirement: Metrics config section
The `AppConfig` SHALL accept an optional `[metrics]` section with a single field `listen_addr` (string, default `"127.0.0.1:9090"`). The section SHALL be optional; absence means metrics are not exposed. The field SHALL be validated to be a valid `host:port` string at config-load time.

#### Scenario: Metrics section absent
- **WHEN** the config file does not contain `[metrics]`
- **THEN** `AppConfig::metrics_config()` returns `None` and no metrics endpoint is bound

#### Scenario: Metrics section with default addr
- **WHEN** the config contains `[metrics]` with no `listen_addr` field
- **THEN** `listen_addr` defaults to `"127.0.0.1:9090"`

#### Scenario: Invalid listen_addr rejected
- **WHEN** the config contains `[metrics] listen_addr = "not-a-socket-addr"`
- **THEN** `AppConfig::load` returns an `OrgaError::ConfigError` mentioning the invalid listen address

## MODIFIED Requirements

### Requirement: Daemon-only metrics endpoint
The `/metrics` HTTP endpoint SHALL be bound only when **both** of the following are true: (a) the CLI was invoked in daemon mode (`orga agent` without `--once`), and (b) the config contains a `[metrics]` section. If either condition is false, no port SHALL be bound. The endpoint SHALL serve the OpenMetrics text format produced by `AgentMetrics::encode()` with content type `text/plain; version=0.0.4; charset=utf-8`. The implementation SHALL use a one-shot TCP listener: each accepted connection is read until the request headers are drained, the encoded body is written with `Connection: close`, and the connection is closed. Keep-alive is not required and SHALL NOT be implemented. No `/healthz` endpoint SHALL be served; the previous `/healthz` route that returned a static `ok` body is removed. The bind address SHALL come from `[metrics].listen_addr`; the default SHALL be `127.0.0.1:9090`. If the bind fails (port in use, permission denied), the daemon SHALL log a warning via the existing `Logger` and continue without metrics — the daemon SHALL NOT crash.

#### Scenario: Daemon with [metrics] binds port
- **WHEN** `orga agent` is invoked with a config that contains `[metrics] listen_addr = "127.0.0.1:9090"`
- **THEN** a TCP listener is bound on `127.0.0.1:9090` before the first poll cycle and `GET /metrics` returns a 200 with the recorder's text output

#### Scenario: /healthz is not served
- **WHEN** `orga agent` is invoked with a config that contains `[metrics]`
- **THEN** a `GET /healthz` request receives a connection-close with no 200 response (the route does not exist; the connection is closed after the request line is read)

#### Scenario: --once mode does not bind port
- **WHEN** `orga agent --once` is invoked
- **THEN** no TCP listener is bound, regardless of whether `[metrics]` is present in the config

#### Scenario: Missing [metrics] does not bind port
- **WHEN** `orga agent` is invoked in daemon mode and the config has no `[metrics]` section
- **THEN** no TCP listener is bound and the daemon runs unchanged

#### Scenario: Bind failure is non-fatal
- **WHEN** `orga agent` is invoked in daemon mode with `[metrics]` configured but the port is already in use
- **THEN** a `WARN` entry is written to the structured log file describing the bind error, no metrics endpoint is served, and the daemon continues processing tickets
