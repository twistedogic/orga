## Why

The agent loop generates operational data — LLM token spend, tool call volume, error rates, and per-ticket processing time — that is currently only available in the structured log file. When running orga in daemon mode (production-like deployments via systemd), there is no way to alert on rising LLM error rates, track token cost across tickets, observe tool-call error spikes, or measure how long ticket processing takes. Sifting through `orga.log` is the only option, which doesn't scale.

Prometheus is the de-facto standard for these metrics. Adding a `/metrics` endpoint bound only in daemon mode gives operators (and developers running a long-lived orga) a queryable, scrapeable view of agent health.

## What Changes

- New `src/metrics.rs` module defining `AgentMetrics` — a `prometheus::Registry`-backed recorder with typed counters and histograms
- New `[metrics]` config section — `listen_addr` (default `127.0.0.1:9090`); absent means metrics are not exposed
- HTTP server bound **only in daemon mode** (no `--once` server). `axum` 0.7 powers a two-route app: `GET /metrics` (OpenMetrics text) and `GET /healthz` (plain `ok`)
- `run_llm_loop` extended to record LLM request count, error count, duration histogram, and token usage (input/output/cached/reasoning) with `model`, `provider`, `agent` labels
- Tool-dispatch closures in main agent, subagent, and sleep-time loops record tool call counts and errors with `tool`, `scope` (`main|subagent|sleep`), and `outcome` (`ok|error`) labels
- `process_ticket` wrapped with a duration histogram; `ticket_processing_duration_seconds{outcome}` records `success|error|skipped|cap_reached`
- New `LlmError` variant on `OrgaError` carries a bounded `error_kind` (`network|rate_limit|auth|parse|backend|other`) so the LLM error counter has stable labels

## Capabilities

### New Capabilities

- `agent-metrics`: Prometheus instrumentation for LLM requests/errors/duration, token usage, tool call counts/errors, and ticket processing duration; HTTP `/metrics` and `/healthz` exposed in daemon mode only

### Modified Capabilities

- `config`: New optional `[metrics]` section with `listen_addr` field
- `agent-loop`: Ticket processing wrapped with timing; tool dispatch observes metrics
- `llm-agent-loop`: `run_llm_loop` observes LLM request metrics, classifies errors into a bounded `error_kind` set
- `error`: New `LlmError { kind, message }` variant so LLM failures can be classified at the call site without string-sniffing

## Impact

- `Cargo.toml` — add `prometheus = "0.13"`, `axum = "0.7"`
- `src/metrics.rs` — new module (~250 lines including tests)
- `src/config.rs` — `MetricsConfig` struct, `AppConfig::metrics()` factory, `validate` accepts the new section
- `src/agent/mod.rs` — `run_daemon` binds the metrics server in a `tokio::spawn`; `run_once` and `process_ticket` observe ticket timing
- `src/agent/loop_runner.rs` — `run_llm_loop` records LLM request metrics, classifies rig-core errors
- `src/agent/mod.rs` dispatch closures (main, subagent, sleep-time) — record tool call metrics
- `src/error.rs` — new `LlmError` variant with `LlmErrorKind` enum
- `src/main.rs` — no changes (server is started inside `run_daemon`)
- `src/lib.rs` — export new `metrics` module
- New tests in `src/metrics.rs` for the `AgentMetrics` recorder
- No CLI flag changes; no breaking config changes
