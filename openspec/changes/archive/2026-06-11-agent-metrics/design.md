## Context

Orga's daemon mode (`orga agent` without `--once`) runs the agent loop continuously, processing tickets from the configured board. Today, every observable signal — LLM token usage, tool call volume, error rates, ticket processing duration — lives only in `orga.log` (structured file logger). Operators have no way to:

- Alert on rising LLM error rates or token spend
- Track tool-call error spikes per tool or per scope (main/subagent/sleep)
- Measure how long ticket processing takes and how that varies
- Distinguish transient vs. systemic backend failures (rate-limit vs. auth vs. network)

Prometheus is the de-facto instrumentation standard. Adding a scrape endpoint bound only in daemon mode gives operators (and developers running a long-lived orga locally) a queryable view of agent health. `--once` mode never binds a port: it processes the current queue and exits, so an HTTP server would have a window too small to be useful and would require a port-release lifecycle on every CLI invocation.

## Goals / Non-Goals

**Goals:**
- Expose LLM request count, error count, duration histogram, and token usage (input / output / cached / reasoning) with stable `model`, `provider`, `agent` labels
- Expose tool call count and error count with `tool`, `scope` (`main` | `subagent` | `sleep`), and `outcome` labels
- Expose ticket processing duration histogram with `outcome` (`success` | `error` | `skipped` | `cap_reached`)
- HTTP `/metrics` endpoint serving OpenMetrics text format; `/healthz` for liveness
- Bind server only in daemon mode, only when `[metrics]` is present in config
- Bounded-cardinality `error_kind` label so a Prometheus dashboard can group errors meaningfully without label explosion
- No new mandatory dependencies for users who do not configure `[metrics]`
- All existing tests remain green

**Non-Goals:**
- Distributed tracing (OpenTelemetry spans, trace_id correlation)
- Pushgateway support (daemon already runs long enough for pull scraping)
- Per-call body capture or LLM prompt/response logging (the structured logger already covers that)
- OTLP / statsd / other exporters (Prometheus only for v1; the recorder is decoupled from the exporter so an OTLP exporter could be added later by swapping the HTTP layer)
- Per-user / per-tenant labels (single-agent model)
- Changing any CLI flag, existing config field, or log format

## Decisions

### Crate choice: `prometheus` 0.13 + `axum` 0.7

The `prometheus` crate is the official Rust client — direct, mature, and the smallest thing that does the job. The recorder surface is plain types (`IntCounterVec`, `HistogramVec`, `Registry`) we can pass by `Arc` and call from any layer. The exporter is a `TextEncoder` that we render in an `axum` handler.

**Alternative considered**: `metrics` + `metrics-exporter-prometheus` facade. Rejected — the facade adds two crates and a runtime-builder pattern for one exporter; the indirection is not justified at the current scale. We can migrate later if/when OTLP is added.

**Alternative considered**: hand-rolled `hyper` 1.x server. Rejected — `axum` is built on the same `hyper` and reduces the HTTP code to two route handlers. Marginal size cost for a much cleaner integration with the existing `tokio` runtime.

### Bounded `error_kind` label, classified at the LLM boundary

The current `OrgaError::BackendError(format!("LLM completion error: {e}"))` wraps rig-core's `CompletionError`, which itself wraps `reqwest::Error` for transport failures. Sniffing the formatted string is fragile.

**Decision**: classify at the `run_llm_loop` call site where the rig error is in hand, then wrap it. A new `OrgaError::LlmError { kind: LlmErrorKind, message: String }` variant carries the classification. The runner maps `reqwest::Error::is_timeout` / `is_connect` → `network`, `is_status() == 429` → `rate_limit`, `is_status() == 401/403` → `auth`, `4xx` → `parse` (LLM call shape), `5xx` → `backend`, anything else → `other`. `LlmErrorKind` is a `#[non_exhaustive]` enum so the set is fixed for v1 but can grow.

`run_llm_loop`'s public signature is unchanged: it still returns `Result<(LoopOutcome, String), OrgaError>`. Internally it does the classification and the metric increment in one place, then constructs the `LlmError` variant.

### Daemon-only server, opt-in via `[metrics]`

Two independent gates: the CLI mode (daemon vs `--once`) and the presence of `[metrics]` config. Either being false ⇒ no port bound. This makes "only daemon exposes metrics" a structural property, not a config accident. A `--once` invocation with `[metrics]` configured still skips the server, because a port opened and closed within one poll cycle is not useful for Prometheus scraping and would complicate graceful shutdown.

**Server lifecycle**:
- In `run_daemon`, before entering the poll loop, build `AgentMetrics` and an `axum` `Router` with two routes
- Bind `TcpListener` to `[metrics].listen_addr` (default `127.0.0.1:9090`)
- Spawn `axum::serve(listener, app)` in a `tokio::spawn` task
- If the bind fails (port in use, permission denied), log a warning via the existing `Logger` and continue the daemon without metrics — never crash the daemon over a metrics failure

### AgentMetrics passed by `Arc`, never as a global

A `OnceLock<AgentMetrics>` global is tempting but the runner's existing pattern is to thread `Arc<Logger>` through call sites. Following that pattern: `AgentMetrics` is wrapped in `Arc<AgentMetrics>`, constructed once in `run_daemon`, and passed into `process_ticket` → `run_llm_loop`. The recorder methods are called via `Arc::clone` in dispatch closures, mirroring the existing logger pattern. The default value (no-op recorder) is used in `--once` mode and in test contexts.

### `make_completion_request` and dispatch closures receive the recorder

The cleanest seam is to give `run_llm_loop` an `Arc<AgentMetrics>` parameter alongside `model` and `history`. The function:
- Starts an `Instant::now()` timer
- On `model.completion(req).await` success: `record_llm_request(ok, model, provider, agent)`, then `add_tokens(usage, ...)` from `response.usage`
- On error: classify into `LlmErrorKind`, construct `OrgaError::LlmError`, `record_llm_error(kind, model, provider, agent)`, return `Err`
- Returns the existing `Result<(LoopOutcome, String), OrgaError>`

The tool-dispatch closure in `agent/mod.rs` (main agent), `run_subagent_loop` (subagent), and `run_sleep_time_agent` (sleep-time) is the other call site. The closure signature is `Fn(String, String, &[AssistantContent]) -> (String, bool)` — it gets a cloned `Arc<AgentMetrics>` and records `record_tool_call(name, scope, outcome)` after computing the result. `outcome` is `error` if the result string starts with `"error:"`, else `ok` — consistent with the existing `agent-tools` spec which already returns errors in that shape.

### Token usage granularity

Five distinct series:
- `orga_llm_tokens_total{kind="input"}`
- `orga_llm_tokens_total{kind="output"}`
- `orga_llm_tokens_total{kind="cached"}` (sum of `cached_input_tokens` + `cache_creation_input_tokens`)
- `orga_llm_tokens_total{kind="reasoning"}`
- `orga_llm_tokens_total{kind="total"}` (the provider's own `total_tokens`, useful for cross-check)

Folding `cached` and `cache_creation` into a single `cached` series is intentional — both are "tokens served from / written to a provider cache" and operators want the combined cache hit-rate view, not the split.

### Ticket processing outcome labels

Four outcomes derived from existing fields in `process_ticket`:
- `success` — `process_ticket` returned `Ok(())`
- `error` — returned `Err(e)`
- `skipped` — the ticket was in `actionable` but the LLM cycle ended with no tool calls (no work to do)
- `cap_reached` — the LLM cycle hit `max_actions_per_ticket` without `done` / `skip`

These are derived in the existing call site `for summary in actionable { process_ticket(...).await }` and recorded once per ticket.

### Default `[metrics]` config

```
[metrics]
listen_addr = "127.0.0.1:9090"
```

`listen_addr` is the only field. Missing `[metrics]` means no server. Future fields (`namespace`, `subsystem`, additional endpoint paths) can be added without breaking existing configs because the section is `Option<MetricsConfig>`.

## Risks / Trade-offs

- **Bind failure silent** — if `[metrics]` is configured but the port cannot be bound, the daemon continues without metrics. This is a deliberate trade: orga-as-agent is the priority; orga-as-observability-target should never break orga-as-agent. Operators see the warning in `orga.log`. Mitigation: structured warning with the bind error and the address attempted.
- **Label cardinality** — `model` and `provider` are bounded (single-digit each in practice). `agent` is single-valued. `tool` is the existing tool-set (~12 names). `scope` is 3. All safe.
- **Two new runtime deps** — `prometheus` and `axum` add a few hundred KB to the binary. Acceptable: orga already has `reqwest` (transitive `hyper` + `tokio` + `tower`), so the new surface is small.
- **Test coverage gap** — the HTTP server is not unit-tested (it would need an async test runtime and a port allocator). The recorder itself is fully unit-tested. The server is small enough that manual verification with `curl localhost:9090/metrics` during a daemon smoke test is the right level.
- **No streaming support** — rig-core's streaming API returns a different type and the existing code uses the non-streaming `completion(req).await`. Metrics observe at the same boundary; no streaming-specific code path is added.

## Migration Plan

Pure addition. Rollout:
1. Add `prometheus` and `axum` to `Cargo.toml`
2. Add `LlmError` variant to `OrgaError` with `LlmErrorKind` enum
3. Add `MetricsConfig` to `AppConfig`; update validation
4. Create `src/metrics.rs` with `AgentMetrics` and unit tests
5. Wire `AgentMetrics` through `run_daemon` → `process_ticket` → `run_llm_loop`
6. Wire tool-dispatch observation in main, subagent, and sleep-time dispatch closures
7. Bind axum server in `run_daemon` when `[metrics]` is present
8. `cargo test` green; manual smoke test with `curl localhost:9090/metrics`

Rollback: revert the commits; no schema changes, no config required, no data migrations.

## Open Questions

- None — all decisions resolved. Implementation can proceed directly from tasks.
