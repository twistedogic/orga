## Context

The agent loop is structured as three nested functions (`run_once_with_client` → `run_once` → `process_ticket`) that each take a tuple of `(config, logger, metrics, dry_run, llm_cfg)`. The same five values are cloned, re-bound, and re-threaded through every layer. The metrics endpoint is served by an axum `Router` with two routes (`/metrics`, `/healthz`) for what is functionally a one-shot text response. The agent's main-tool set is enumerated in three places (Rust `Vec<String>` in `mod.rs`, prose string in `context.rs` non-dispatcher prompt, prose string in `context.rs` dispatcher prompt) that have already drifted out of sync — the non-dispatcher prose advertises `create_sub` and `bash` that the code path does not actually expose. The history is scanned twice via `serde_json::to_string(c).unwrap_or_default().contains("\"type\":\"toolresult\"")` patterns to count actions and recover a subagent return value, when both pieces of data are already typed in `Message::User { content: OneOrMany<UserContent> }`. Error classification lives in two places: `From<reqwest::Error>` in `error.rs` and a separate `classify_llm_error` ladder in `loop_runner.rs` doing overlapping work.

Constraints:
- Must not change metric names, labels, or text format (Prometheus dashboards and alerts depend on them).
- Must not change the `run_llm_loop` public signature in a way that breaks existing callers (only `mod.rs` calls it today, so the signature can tighten).
- Must keep `cargo test --lib` green; no behavioral changes for tests.
- Must preserve the agent's ability to be invoked in `--once` and daemon modes.

## Goals / Non-Goals

**Goals:**
- Reduce `src/agent/mod.rs` line count by ~250 lines without changing behavior.
- Remove the `axum` direct dependency and its transitive tree.
- Single source of truth for the main-agent tool set (Rust constant + system-prompt prose derived from it).
- One classification site for LLM errors, one place that turns rig-core `CompletionError` into `OrgaError`.
- Type-driven history inspection: no `serde_json::to_string` round-trips for control flow.
- System prompts as `.md` files in `src/agent/prompts/`, embedded at compile time via `include_str!`.
- Tests still pass; new tests only for genuinely new code paths (the typed history helpers).

**Non-Goals:**
- No changes to metric names, label sets, or histogram buckets.
- No new metric series.
- No changes to the `--once` CLI flag, the daemon poll loop interval, or the board backend trait.
- No migration of user-facing config; the `[metrics]` schema is unchanged.
- No new public API; the change is internal-only.
- No restructure of `src/board/`, `src/memory/`, or the SQLite layer.

## Decisions

**D1. Replace axum with a hand-rolled one-shot TCP listener, not with `prometheus::push` or with `hyper` directly.**

`prometheus::push` requires an external Pushgateway — adds a deployment dependency the user did not ask for. `hyper` direct is essentially rewriting axum for one route. A hand-rolled listener that accepts a connection, drains the headers, writes the encoded body with `Connection: close`, and closes the socket is ~25 lines of tokio code and serves the same Prometheus contract. Prometheus scrapers open one connection per scrape and do not reuse it, so HTTP/1.1 keep-alive is unused capacity.

The `/healthz` endpoint is removed. It returned `"ok"` regardless of agent health (no check that the daemon loop is making progress, no check that the last poll succeeded). A misleading health probe is worse than no health probe; an external watchdog tailing the daemon's `orga.log` is the standard pattern.

**D2. Bundle the per-run parameter tuple into a `RunContext { config, logger, metrics, dry_run, llm_cfg }` struct.**

Three functions (`run_ticket`, `run_sleep_time_agent`, `run_defrag_agent`) take the same five values. A struct passed by reference drops ~30 `Arc::clone(&logger)` shadow-bindings and two `#[allow(clippy::too_many_arguments)]` annotations. The struct's `llm_cfg: &LlmConfig` field is borrowed from `config.llm` so callers don't pass it separately.

**D3. Use `include_str!("prompts/<name>.md")` for system prompts, with `{placeholder}` substitution.**

The four system prompts (sleep-time, defrag, main-agent, dispatcher) are content, not code. They change less often than the code around them, benefit from `.md` rendering in PRs, and have a single author responsible for prompt wording. `include_str!` embeds the file at compile time — no runtime FS lookup, no risk of the binary shipping without the prompt. `{placeholder}` substitution (vs. a real template engine like `handlebars` or `tera`) keeps the code dependency-free; the four prompts together have ~6 placeholders.

**D4. One `pub const MAIN_TOOLS: &[&str]`, consumed by both `tool_definitions_for(...)` and the system-prompt prose.**

The prose uses `MAIN_TOOLS.join(", ")` for the "Available tools: …" line. The Rust code uses `tool_definitions_for(MAIN_TOOLS)`. Adding a tool is now a one-line change in `tools.rs`. The current drift (prose mentions `create_sub, bash`; code does not expose them) is fixed by deleting the prose override and using the constant directly.

The two previously separate "main-agent" and "dispatcher" prompts end up advertising the same tool set. That is intentional: both code paths use the same `tool_definitions_for(MAIN_TOOLS)` call today; the difference was only in the prose.

**D5. Widen `tool_definitions_for` from `&[String]` to `&[&str]`.**

Three call sites (`mod.rs` main agent, `mod.rs` subagent, `tools.rs` defrag). All three would prefer string literals; all three today build `Vec<String>` to satisfy the signature. Widening drops the allocations and lets `MAIN_TOOLS` flow in without conversion. The test fixture updates from `vec!["comment".to_string(), …]` to `["comment", …]`.

**D6. Typed history inspection via `match` on `Message::User` / `Message::Assistant` content.**

The two `serde_json::to_string(c).unwrap_or_default().contains("…")` blocks are replaced with:
- `count_actions_and_detect_done(history: &[Message]) -> (usize, bool)` — typed fold counting `UserContent::ToolResult` variants and detecting `AssistantContent::ToolCall` with `name == "done"`.
- `extract_return_value(msg: &Message) -> Option<String>` — typed walk through `UserContent::ToolResult(ToolResult).content → ToolResultContent::Text(Text).text`.

The `"tool_call_id"` substring fallback is removed: the only writer to history is `Message::tool_result(...)` in `loop_runner.rs:116`, which always produces `UserContent::ToolResult`. The defensive cargo-cult matches a non-existent case.

**D7. Single classification site for LLM errors.**

Move `classify_llm_error` / `classify_http_client_error` / `status_to_kind` out of `loop_runner.rs` into a `From<CompletionError> for OrgaError` impl. Delete the duplicate `From<reqwest::Error>` timeout/connect branch in `error.rs` (the new impl covers it). The four `tests` in `loop_runner.rs:215-251` collapse to a single round-trip test that constructs a `CompletionError` and asserts the resulting `OrgaError::LlmError { kind, .. }`.

The `LlmErrorKind::Display` impl is deleted; `metrics.rs:169` and `loop_runner.rs` callers switch to `kind.as_str()`. The `is_llm_error_kind` method on `OrgaError` is deleted (zero callers in `src/`).

**D8. Drop the `[artifact]` migration shim.**

`config.rs:159-163` rejects configs that contain a `[artifact]` section, with a pointer to the new `[workspace]` section. This was a migration aid added when `[workspace]` replaced `[artifact]`. Users who hit it have migrated; the shim is dead code. Drop it next minor release.

## Risks / Trade-offs

**[Risk] Hand-rolled HTTP listener may behave subtly differently from axum on edge-case clients.** → Tested by `cargo test` running the existing integration suite. Prometheus scrapers use a single canonical request shape (`GET /metrics HTTP/1.1\r\n\r\n`); we don't parse the request, we just drain and respond. No client in the supported set is known to fail.

**[Risk] `[artifact]` migration shim removal breaks a user mid-migration.** → Acceptable: the shim has been in place since the previous release; users who have not migrated are a smaller and smaller cohort over time. If a regression report comes in, the shim is two lines to restore.

**[Risk] `include_str!` makes prompts harder to test in isolation.** → Prompts are content; the test that matters is "does the agent behave correctly with this prompt," which is end-to-end, not unit-level. Unit-testing prompt text is testing the constant itself, which is trivial.

**[Risk] Unifying `From<CompletionError>` and removing the `From<reqwest::Error>` impl could regress a code path that relied on the latter's blanket `?` operator.** → Mitigated by `cargo build` failing at the `?` site; the regression would be caught immediately and the fix is adding the missing `From` back. The two impls had overlapping coverage anyway (both classified timeout/connect as `network`).

**[Risk] `MAIN_TOOLS` constant loses the "main vs dispatcher tool set" distinction.** → The current code already uses one tool set; the prose was the only thing that diverged. The previous intent (if any) of "dispatcher sees fewer tools" was not implemented in code. If that intent resurfaces, it becomes an ADDED spec, not a hidden assumption.

**[Risk] `extract_return_value` assumes `Message::tool_result(...)` produces `UserContent::ToolResult(ToolResult)` whose first content is `ToolResultContent::Text`.** → Confirmed against rig-core 0.37.0 source. If a future rig-core version changes the variant or the inner enum, the typed match fails to compile, which is exactly the behavior we want.

## Migration Plan

Single PR. No runtime migration: the binary drop-in replaces the existing one, restarts, and resumes polling. The only behavior change visible to an operator is:
- `/healthz` no longer responds. External liveness probes must be updated to either tail `orga.log` or check that the daemon process is alive.
- The metrics scraper keeps working unchanged.

Rollback: revert the PR. No schema changes, no DB migrations, no config edits.

## Open Questions

None. All decisions resolve to existing patterns in the codebase (`include_str!` already used by `skills/orga/SKILL.md` semantics; `Arc<Logger>` bundles already used by `run_daemon`; `const &[&str]` already used by `tools.rs` `VALID_TOOLS`).
