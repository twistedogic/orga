## 1. Add dependencies

- [x] 1.1 Add `prometheus = "0.13"` and `axum = "0.7"` to `[dependencies]` in `Cargo.toml`
- [x] 1.2 Run `cargo check` — dependencies resolve and compile

## 2. Add LlmError variant to OrgaError

- [x] 2.1 Add `LlmErrorKind` enum (`network`, `rate_limit`, `auth`, `parse`, `backend`, `other`) in `src/error.rs`; derive `Debug, Clone, Copy, PartialEq, Eq`
- [x] 2.2 Add `LlmError { kind: LlmErrorKind, message: String }` variant to `OrgaError`; mark `LlmErrorKind` `#[non_exhaustive]`
- [x] 2.3 Add `OrgaError::is_llm_error_kind(&self, kind: LlmErrorKind) -> bool` helper for callers that want to branch on classification
- [x] 2.4 Add unit tests covering `LlmError` Display output, `LlmErrorKind` Display (snake_case label), and the new helper

## 3. Add MetricsConfig to AppConfig

- [x] 3.1 Add `MetricsConfig { listen_addr: Option<String> }` to `src/config.rs`; default `listen_addr` to `"127.0.0.1:9090"` via a `listen_addr()` method
- [x] 3.2 Add `metrics: Option<MetricsConfig>` field to `AppConfig`; update all `AppConfig { ... }` literals in `init.rs`, `agent/context.rs` with `metrics: None`
- [x] 3.3 Add `AppConfig::metrics_config() -> Option<&MetricsConfig>` accessor
- [x] 3.4 Validate `listen_addr` is a valid `SocketAddr`; reject configs with `#[metrics] listen_addr = "not-a-socket-addr"`
- [x] 3.5 Add unit tests for absent section, default addr, custom addr, invalid addr rejection

## 4. Create src/metrics.rs

- [x] 4.1 Define `pub struct AgentMetrics` with a public `new()` constructor that registers all six metrics
- [x] 4.2 Implement `record_llm_request(model, provider, agent)` and `record_llm_error(model, provider, agent, kind)`
- [x] 4.3 Implement `record_llm_duration(model, provider, agent, elapsed: Duration)` with buckets from 0.05s to 60s
- [x] 4.4 Implement `record_tokens(model, provider, agent, usage: &rig_core::completion::Usage)` adding five `kind` series
- [x] 4.5 Implement `record_tool_call(tool, scope: ToolScope, outcome: ToolOutcome)` and `record_ticket(outcome: TicketOutcome, elapsed: Duration)`
- [x] 4.6 Implement `pub fn encode(&self) -> Result<String, prometheus::Error>` using `TextEncoder`
- [x] 4.7 Add unit tests for: registration, counter increments, error kind label, token aggregation, zero-usage noop, tool scope/outcome, ticket histogram, encode output

## 5. Wire AgentMetrics into run_llm_loop

- [x] 5.1 Extend `run_llm_loop` signature to take `metrics: Arc<AgentMetrics>`, `model_label: &str`, `provider: &str`, `agent: &str`
- [x] 5.2 Wrap the `model.completion(req).await` call in `Instant::now()` and observe duration on both success and error paths
- [x] 5.3 On success: `record_llm_request(ok)` and `record_tokens(usage)`
- [x] 5.4 On error: classify the rig `CompletionError` via `classify_llm_error`; construct `OrgaError::LlmError { kind, message }`; `record_llm_error(kind)`; return the new variant
- [x] 5.5 Classifier inspects `http_client::Error::Instance(boxed reqwest::Error)` and `InvalidStatusCodeWithMessage(status, _)` to produce a bounded `LlmErrorKind`
- [x] 5.6 Add unit tests for `status_to_kind` mapping and `classify_llm_error` for `ResponseError` / `ProviderError` / `InvalidStatusCodeWithMessage`

## 6. Wire tool-dispatch observation

- [x] 6.1 In `process_ticket`'s dispatch closure, clone the `Arc<AgentMetrics>`, compute outcome from result string (`error:` prefix → `Error`; else `Ok`), and call `record_tool_call(name, ToolScope::Main, outcome)` before returning
- [x] 6.2 In `run_subagent_loop`'s dispatch closure, do the same with `ToolScope::Subagent`
- [x] 6.3 In `run_sleep_time_agent` and `run_defrag_agent` dispatch closures, do the same with `ToolScope::Sleep`
- [x] 6.4 `Arc<AgentMetrics>` is threaded into all three closures via `Arc::clone` in captures (matching the existing `logger: Arc<Logger>` pattern); `ToolContext` is unchanged

## 7. Wire ticket processing duration

- [x] 7.1 In `run_once_with_client`'s `for summary in actionable { ... }` loop, wrap `process_ticket(...).await` with `Instant::now()` and call `record_ticket(outcome, elapsed)` after the await resolves
- [x] 7.2 Map outcomes: `Ok(TicketProcessingOutcome::Success)` → `Success`; `Err(_)` → `Error`; `Ok(Skipped)` → `Skipped`; `Ok(CapReached)` → `CapReached`
- [x] 7.3 `process_ticket` returns `Result<TicketProcessingOutcome, OrgaError>`; `TicketProcessingOutcome::from_loop_outcome` maps `LoopOutcome` → `TicketProcessingOutcome`

## 8. Bind metrics server in run_daemon

- [x] 8.1 In `run_daemon`, before entering the poll loop, build `AgentMetrics::new()` wrapped in `Arc` and clone it into `run_once_with_client`
- [x] 8.2 When `config.metrics_config().is_some()`, build an `axum::Router` with two routes: `GET /metrics` returning `AgentMetrics::encode()` as `text/plain; version=0.0.4; charset=utf-8`, and `GET /healthz` returning `ok` with content type `text/plain`
- [x] 8.3 Bind a `tokio::net::TcpListener` to `listen_addr`; on success, spawn `axum::serve(listener, app)` in `tokio::spawn`; on failure, log a warning via `Logger` and continue without the server
- [x] 8.4 Confirm `--once` mode does not bind the server (server-start code is inside `run_daemon`)

## 9. Update lib.rs and final wiring

- [x] 9.1 Add `pub mod metrics;` to `src/lib.rs`
- [x] 9.2 Run `cargo test` — all 178 lib tests + 30 integration tests pass
- [x] 9.3 Run `cargo clippy` — no new warnings introduced
- [x] 9.4 Smoke test `agent::tests::bind_metrics_server_serves_text_and_healthz` boots a real axum server, fetches `/metrics` and `/healthz`, verifies both responses
