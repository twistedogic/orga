## Why

The agent loop has accumulated duplication and over-engineering across `src/agent/` modules that the existing specs do not yet capture as requirements. The diff currently in flight introduces a fresh layer of this: an axum + prometheus HTTP server for a single counter set, a 600-line process_ticket that threads `(config, logger, metrics, dry_run)` through every layer via `Arc::clone`, prompt text duplicated between Rust string literals and re-emitted JSON via `serde_json::to_string` substring matches, and a hand-built list of "main agent tools" duplicated in three places (Rust code list, two prose strings in the system prompt) that have already drifted apart.

This change is a pure refactor: external behavior (board interaction, prompt semantics, metric names, metric format, scrape endpoint) is preserved or tightened; internal complexity is reduced.

## What Changes

- **Replace axum-based metrics server with a hand-rolled one-shot TCP listener.** The Prometheus text format and `Content-Type: text/plain; version=0.0.4` header are preserved; `axum` is removed as a dependency. The `/healthz` endpoint is removed (it reported `"ok"` regardless of agent state and no caller depended on it). (`src/agent/mod.rs:62-112`, `Cargo.toml:12`.)
- **Collapse `run_once_with_client` / `run_once` / `process_ticket` into a single `run_ticket` function** that takes a `RunContext { config, logger, metrics, dry_run, llm_cfg }` bundle. Eliminates one layer of indirection, ~30 `Arc::clone` sites, and a duplicate `build_board` call per ticket. (`src/agent/mod.rs:114-412`.)
- **Apply the same `RunContext`-style parameter bundling to `run_sleep_time_agent` and `run_defrag_agent`.** Removes the `#[allow(clippy::too_many_arguments)]` annotations. (`src/agent/mod.rs:630-742`, `:744-…`.)
- **Hoist inlined system prompts to `include_str!` markdown files.** The sleep-time, defrag, main-agent, and dispatcher system prompts move from Rust `format!(…)` literals to `src/agent/prompts/*.md` with `{placeholder}` substitution. (`src/agent/mod.rs:660-669`, `:766-780`; `src/agent/context.rs:80-105`.)
- **Introduce `pub const MAIN_TOOLS: &[&str]` in `src/agent/tools.rs`.** The same constant drives both the `tool_definitions_for(...)` call in code and the "Available tools: …" prose in the system prompt, closing a drift bug where the prose advertised `create_sub` and `bash` that the code path did not actually expose. (`src/agent/mod.rs:249-260`; `src/agent/context.rs:86`, `:98`.)
- **Widen `tool_definitions_for(names: &[String])` to `&[&str]`.** Removes `.to_string()` allocations at every call site and lets `MAIN_TOOLS` flow in without an intermediate `Vec<String>`. (`src/agent/tools.rs:59`; `src/agent/mod.rs:261`, `:527`, `:665`.)
- **Replace JSON substring matches in history scanning with typed `match` on `Message::User`/`Message::Assistant` content.** The two `serde_json::to_string(c).unwrap_or_default().contains("\"type\":\"toolresult\"")` blocks become `matches!(c, UserContent::ToolResult(_))`; the subagent return-value extraction becomes a typed fold over `ToolResultContent::Text`. (`src/agent/mod.rs:349-375`, `:602-615`.)
- **Drop unused `LlmErrorKind::Display` impl and `OrgaError::is_llm_error_kind` helper.** The helper has zero callers in `src/` (only a test for it); the `Display` impl is replaced with `LlmErrorKind::as_str() -> &'static str`. (`src/error.rs:14-26`, `:62-65`.)
- **Unify LLM error classification into a single `From<CompletionError> for OrgaError`.** Delete the duplicate `From<reqwest::Error>` timeout/connect logic in `src/error.rs:67-75` and the matching `classify_llm_error` / `classify_http_client_error` / `status_to_kind` helpers in `src/agent/loop_runner.rs:139-181`. One classification site, one set of tests.
- **Drop the post-decode text scan that rejects removed `[artifact]` config sections** (`src/config.rs:159-163`). Migration shim for a config section removed in the previous release; users who hit it have already migrated.
- **Table-drive `AppConfig::validate`** and merge the trello/linear init flows into one parameterized helper. (`src/config.rs:216-304`; `src/init.rs:147-367`.)

No public API is removed. The only user-visible behavior changes are: `/healthz` is no longer served on the metrics listen address (Prometheus scrapers do not hit it), and the prose listing of available tools in the system prompt now exactly matches the tools the code actually exposes.

## Capabilities

### New Capabilities
None. This change is a refactor with no net-new user-facing capability.

### Modified Capabilities

- `agent-metrics`: the `/metrics` endpoint now serves one-shot HTTP/1.0 responses (connection-close per scrape) instead of axum's keep-alive router; the `/healthz` endpoint is removed. The Prometheus text format, the `Content-Type` header, the `listen_addr` config field, and the metric names and labels are unchanged. Scrapers see the same body bytes.
- `agent-tools`: the set of tools advertised in the agent's system prompt must exactly equal the set of tools the agent code path can dispatch. Today both come from a single `pub const MAIN_TOOLS: &[&str]` in `src/agent/tools.rs`.

## Impact

- **Code**: `src/agent/mod.rs`, `src/agent/context.rs`, `src/agent/tools.rs`, `src/agent/loop_runner.rs`, `src/error.rs`, `src/config.rs`, `src/init.rs`. Net line count expected to drop by 600–700 lines.
- **Dependencies**: `axum = "0.7"` removed from `Cargo.toml`. `prometheus = "0.13"` retained (still used for encoding). Transitive deps (`tower`, `hyper`, `http-body-util`, `http`, `mime`, …) drop out of the lockfile.
- **Files added**: `src/agent/prompts/{sleep_time,defrag,main_agent,dispatcher}.md` — system prompts as reviewable `.md`.
- **No spec changes for end users** of the orga CLI: same commands, same config schema, same metric names, same scrape contract (modulo the documented `/healthz` removal).
- **Build / test**: `cargo build` and `cargo test --lib` must continue to pass. Existing test fixtures exercise the affected functions.
