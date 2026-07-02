## 1. Replace metrics HTTP server with hand-rolled listener

- [x] 1.1 Remove `axum = "0.7"` from `Cargo.toml`; run `cargo build` to confirm `prometheus` and tokio are sufficient
- [x] 1.2 Replace `bind_metrics_server` body in `src/agent/mod.rs:62-112` with the hand-rolled one-shot TCP listener: `tokio::net::TcpListener::bind`, `accept()` loop spawning one `serve_one` task per connection, `serve_one` drains request bytes via `AsyncReadExt::read`, writes `HTTP/1.0 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: <n>\r\nConnection: close\r\n\r\n<body>` via `AsyncWriteExt::write_all`, then `shutdown()`
- [x] 1.3 Remove the `healthz()` async fn and the `.route("/healthz", get(healthz))` line
- [x] 1.4 Verify `cargo test --lib` is still green (no test referenced `/healthz`; confirm by grep)

## 2. Hoist system prompts to `.md` files

- [x] 2.1 Create `src/agent/prompts/sleep_time.md` with the current sleep-time system prompt text; replace `{tree_index}` interpolation point with the literal token `{tree_index}`
- [x] 2.2 Create `src/agent/prompts/defrag.md` with the current defrag system prompt text; replace `{tree}` with `{tree}`
- [x] 2.3 Create `src/agent/prompts/main_agent.md` with the current main-agent system prompt text; replace `{agent_name}` and `{tools}` placeholders with `{agent_name}` and `{tools}`
- [x] 2.4 Create `src/agent/prompts/dispatcher.md` with the current dispatcher system prompt text; replace `{agent_name}` and `{tools}` placeholders
- [x] 2.5 In `src/agent/mod.rs`, declare `const SLEEP_SYSTEM_PROMPT: &str = include_str!("prompts/sleep_time.md");` and `const DEFRAG_SYSTEM_PROMPT: &str = include_str!("prompts/defrag.md");` near the top; replace the two inline `format!(…)` blocks (lines 660-669 and 766-780) with `.replace("{tree_index}", &tree_index)` / `.replace("{tree}", &tree)`
- [x] 2.6 In `src/agent/context.rs`, declare `const MAIN_AGENT_SYSTEM_PROMPT` and `const DISPATCHER_SYSTEM_PROMPT` via `include_str!`; replace the two `format!(…)` blocks (lines 80-105) with `.replace("{agent_name}", &app_cfg.agent.name).replace("{tools}", &tools::MAIN_TOOLS.join(", "))`

## 3. Single `MAIN_TOOLS` source of truth

- [x] 3.1 In `src/agent/tools.rs`, add `pub const MAIN_TOOLS: &[&str] = &[ "comment", "dispatch", "skip", "done", "compact", "todos", "memory_list", "memory_read", "memory_write", "memory_search" ];`
- [x] 3.2 Widen `pub fn tool_definitions_for(names: &[String])` to `&[&str]`; update the body to compare `*n == t.name` (drop the `&` on the closure parameter)
- [x] 3.3 In `src/agent/tools.rs:570` (`defrag_tool_definitions`), drop the `.to_string()` calls from the three `"memory_*"` literals
- [x] 3.4 In `src/agent/tools.rs:691` test, change `vec!["comment".to_string(), "done".to_string()]` to `["comment", "done"]`
- [x] 3.5 In `src/agent/mod.rs:249-260`, replace the `let main_agent_tools = vec![…to_string()…]` block with `tool_definitions_for(tools::MAIN_TOOLS)`
- [x] 3.6 In `src/agent/mod.rs:503-509` (subagent loop), collapse the four `.contains`/`.push` blocks into one `for tool in &["return", "todos", "memory_list", "memory_read", "memory_write", "memory_search"]` loop; pass `tool_names.iter().map(String::as_str).collect::<Vec<_>>()` to `tool_definitions_for`
- [x] 3.7 In `src/agent/mod.rs:664-668` (sleep tools), drop the three `.to_string()` calls

## 4. Typed history inspection

- [x] 4.1 In `src/agent/mod.rs`, add a private `fn count_actions_and_detect_done(history: &[Message]) -> (usize, bool)` that walks `&[Message]` and matches `UserContent::ToolResult(_)` for the action count and `AssistantContent::ToolCall(tc) if tc.function.name == "done"` for the done flag; no `serde_json` calls
- [x] 4.2 In `src/agent/mod.rs`, add a private `fn extract_return_value(msg: &Message) -> Option<String>` that matches `Message::User { content } → UserContent::ToolResult(tr) → ToolResultContent::Text(t)` and returns `Some(t.text.clone())`; no `serde_json` calls
- [x] 4.3 Replace the 27-line `for msg in &history { match msg { … serde_json::to_string … } }` block (lines 349-375) with `let (action_count, did_done) = count_actions_and_detect_done(&history);`
- [x] 4.4 Replace the 14-line `if let Some(Message::User { content }) = history.last() { … serde_json::from_str … }` block (lines 602-615) with `let last_return_value = history.last().and_then(extract_return_value);`
- [x] 4.5 Add unit tests in the existing `#[cfg(test)] mod tests` block in `mod.rs` covering: a history containing one tool result increments `action_count` by 1; a history containing `AssistantContent::ToolCall { name: "done", .. }` sets `did_done` to true; `extract_return_value` returns `Some("…")` for a `UserContent::ToolResult(ToolResult { content: OneOrMany::one(ToolResultContent::Text(Text { text: "hi".into() })), .. })` and `None` for any other shape

## 5. Unify LLM error classification

- [x] 5.1 In `src/error.rs`, add `impl From<CompletionError> for OrgaError` containing the classification logic currently in `src/agent/loop_runner.rs:139-181` (transport → `Network`, 429 → `RateLimited`, 401/403 → `Auth`, 4xx → `Parse`, 5xx → `Backend`, else → `Other`); preserve the inner-error downcast and the `record_llm_error` site at the call site of the `From` impl
- [x] 5.2 Delete `classify_llm_error`, `classify_http_client_error`, and `status_to_kind` from `src/agent/loop_runner.rs`
- [x] 5.3 In `src/error.rs`, replace the `From<reqwest::Error>` impl's timeout/connect branch with a passthrough that builds `OrgaError::LlmError { kind: …, message: … }` for the cases the unified classifier handles; if the new `From<CompletionError>` makes the `From<reqwest::Error>` impl redundant (no other `?` site), delete it
- [x] 5.4 Delete `OrgaError::is_llm_error_kind` and its `tests::is_llm_error_kind_matches_exact_variant` test from `src/error.rs` (zero callers in `src/`)
- [x] 5.5 In `src/error.rs`, delete `impl Display for LlmErrorKind`; add `pub fn as_str(&self) -> &'static str` returning the same labels (`"network"`, `"rate_limit"`, `"auth"`, `"parse"`, `"backend"`, `"other"`); update `metrics.rs:169` and any other call sites to use `as_str()` instead of `to_string()`
- [x] 5.6 Consolidate the four `classify_*` unit tests in `src/agent/loop_runner.rs:215-251` into one test that constructs a `CompletionError` and asserts the resulting `OrgaError::LlmError { kind, .. }`; add one test for each `LlmErrorKind` branch (Network, RateLimited, Auth, Parse, Backend, Other)

## 6. Collapse `run_once_with_client` / `run_once` / `process_ticket`

- [x] 6.1 In `src/agent/mod.rs`, declare `pub struct RunContext<'a> { pub config: &'a AppConfig, pub logger: Arc<Logger>, pub metrics: Arc<AgentMetrics>, pub dry_run: bool, pub llm_cfg: &'a LlmConfig, }`
- [x] 6.2 Replace `run_once_with_client` (lines 114-125) with a single `pub async fn run_agent(once, dry_run, config, logger) -> Result<(), OrgaError>` that builds the `LlmClient`, builds a `RunContext { llm_cfg: config.llm_config()?, … }`, and dispatches to either a one-shot path (no daemon loop) or the daemon loop
- [x] 6.3 Collapse `run_once` (lines 127-181) into the one-shot path: filter actionable tickets, iterate, call `run_ticket(&ctx, &client, ticket_id).await`, record outcome
- [x] 6.4 Rewrite `process_ticket` (lines 183-412) as `run_ticket(ctx: &RunContext<'_>, client, ticket_id) -> Result<TicketProcessingOutcome, OrgaError>`; single `build_board` call (passed into `ToolContext`); typed history inspection via the helpers from task 4; sleep-time trigger via `run_sleep_time_agent(ctx, client, &ticket, context_repo)`
- [x] 6.5 Run `cargo build` and `cargo test --lib` after each sub-task; resolve compile errors before continuing

## 7. Bundle params for `run_sleep_time_agent` and `run_defrag_agent`

- [x] 7.1 Collapse `run_sleep_time_agent`'s 11-parameter signature (lines 631-643) to `(ctx: &RunContext<'_>, client: &C, ticket: &Ticket, context_repo: ContextRepository) -> Result<(), OrgaError>`; access `ctx.llm_cfg.model`, `ctx.config.defrag_file_threshold()`, `ctx.config.defrag_size_threshold_kb()`, `ctx.config.memory_repo_path()`, `ctx.config.agent.name` via the bundle
- [x] 7.2 Collapse `run_defrag_agent`'s 7-parameter signature similarly to `(ctx: &RunContext<'_>, client: &C) -> Result<(), OrgaError>`
- [x] 7.3 Remove `#[allow(clippy::too_many_arguments)]` from both functions
- [x] 7.4 Verify `cargo build` and `cargo test --lib` are green

## 8. Drop dead config migration shim

- [x] 8.1 In `src/config.rs:159-163`, delete the `if content.contains("[artifact]") || content.contains("[artifact.git]")` post-decode scan; delete the `OrgaError::ConfigError` branch that returns the migration message
- [x] 8.2 Verify `cargo test --lib` is green (no test exercises the removed branch)

## 9. Verification

- [x] 9.1 `cargo build` clean
- [x] 9.2 `cargo test --lib` — all 178+ existing tests pass
- [x] 9.3 `cargo clippy --all-targets --all-features --locked -- -D warnings` clean (run if configured in this repo; otherwise `cargo clippy --all-targets -- -D warnings`)
- [x] 9.4 `git diff --stat` shows net line count decreased by ≥ 500 lines across the affected files
- [x] 9.5 Grep for `serde_json::to_string` in `src/agent/mod.rs` returns zero hits inside `for msg in &history` or `if let Some(Message::User …)` blocks (i.e., no JSON round-trips in history inspection)
- [x] 9.6 Grep for `axum` in `Cargo.toml` returns zero hits
- [x] 9.7 Grep for `is_llm_error_kind` in `src/` returns zero hits
- [x] 9.8 Grep for `"\"type\":\"toolresult\""` in `src/` returns zero hits
- [x] 9.9 `cargo tree | grep axum` returns zero hits
