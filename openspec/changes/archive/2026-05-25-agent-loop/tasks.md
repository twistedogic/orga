## 1. Dependencies & Config

- [x] 1.1 Add `rig-core`, `tokio` (with `rt`, `macros` features) to `Cargo.toml`
- [x] 1.2 Add `LlmConfig` struct to `src/config.rs` with fields: `provider`, `api_key`, `model`, `endpoint` (optional), `poll_interval_secs` (default 60), `max_actions_per_ticket` (default 10), `max_artifact_inline_bytes` (default 8192)
- [x] 1.3 Add `llm: Option<LlmConfig>` field to `AppConfig`
- [x] 1.4 Add config validation: if `[llm]` present, `provider` must be `"anthropic"` or `"openai"`, `api_key` and `model` must be non-empty
- [x] 1.5 Add helper `AppConfig::llm_config() -> Result<&LlmConfig, OrgaError>` that returns error if `[llm]` is absent
- [x] 1.6 Write config tests: valid `[llm]` section loads, missing `api_key` fails, unknown provider fails, absent section doesn't affect other commands

## 2. Module scaffold

- [x] 2.1 Create `src/agent/mod.rs` — exports `AgentLoop`, `run_agent`
- [x] 2.2 Create `src/agent/config.rs` — `build_llm_client(cfg: &LlmConfig)` returning a rig provider client (Anthropic or OpenAI, with optional endpoint override)
- [x] 2.3 Create `src/agent/tools.rs` — `AgentTool` enum with all tool variants and their input structs
- [x] 2.4 Create `src/agent/context.rs` — `build_context(ticket, memory, artifacts, llm_cfg, app_cfg)` returning system + user message strings
- [x] 2.5 Export `agent` module from `src/lib.rs`

## 3. Tool definitions

- [x] 3.1 Define `AgentTool` enum variants: `Comment`, `MoveTicket`, `Assign`, `CreateSub`, `AddChecklistItem`, `CheckItem`, `SetMemory`, `CommitArtifact`, `GetArtifact`, `Compact`, `Done`, `Skip`
- [x] 3.2 Define JSON schema for each tool (name, description, input properties, required fields) as rig tool definitions
- [x] 3.3 Implement `dispatch_tool(tool: AgentTool, ticket_id, board, memory_store, artifact_store, compaction_store, dry_run) -> String` — returns tool result string; mutating tools check `dry_run` flag
- [x] 3.4 Implement dry-run path for all mutating tools: log action to stdout, return `"[dry-run] <action> would have been executed"`
- [x] 3.5 Implement error path: tool execution errors return descriptive string (not panic); fed back as tool_result

## 4. Context builder

- [x] 4.1 Build system prompt: workflow prompt for ticket's column (if any) + agent name + brief capability summary
- [x] 4.2 Build user message: ticket fields (id, title, description, list, url, creator, assignees), checklists, comments (with compaction block if present)
- [x] 4.3 Append memory: if memory entry exists for ticket, include it in user message
- [x] 4.4 Append artifacts: for each artifact on the ticket, inline content if size ≤ `max_artifact_inline_bytes`, else include metadata only (name, size, committed_at) with note that `get_artifact` can fetch it

## 5. Agent loop

- [x] 5.1 Implement `run_once(config, logger) -> Result<(), OrgaError>`: fetch assigned tickets, filter to actionable (not completed, last_commenter_is_agent = false), process each sequentially
- [x] 5.2 Implement per-ticket cycle: build context → tool-call loop → stop on done/skip/no-tool-calls/cap
- [x] 5.3 Wire `max_actions_per_ticket` cap: track action count per ticket, break loop when reached
- [x] 5.4 Implement error isolation: catch per-ticket errors, log them, continue to next ticket
- [x] 5.5 Implement `run_daemon(config, logger)`: calls `run_once` then sleeps `poll_interval_secs`, loops until SIGINT/SIGTERM
- [x] 5.6 Log per-ticket actions to the orga log file (daemon mode) and to stdout (`--once` mode)

## 6. CLI wiring

- [x] 6.1 Add `Agent { once: bool, dry_run: bool }` variant to `Commands` enum in `src/main.rs`
- [x] 6.2 Add `#[command(about = "Run the agent loop")]` with `--once` and `--dry-run` flags
- [x] 6.3 Dispatch `Commands::Agent` to `run_agent(once, dry_run, &config, logger)`
- [x] 6.4 `run_agent` validates `[llm]` config present before doing anything, exits with clear error if absent

## 7. Integration & testing

- [x] 7.1 Write unit tests for `dispatch_tool` dry-run paths (no board/artifact calls made)
- [x] 7.2 Write unit tests for `build_context`: artifact inlining below cap, metadata-only above cap, memory inclusion, compaction block included
- [x] 7.3 Write unit tests for action cap: cycle stops at `max_actions_per_ticket`
- [x] 7.4 Verify `cargo build` succeeds with new dependencies
- [x] 7.5 Verify existing CLI tests still pass (`cargo test`)
- [x] 7.6 Manual smoke test: `orga agent --once --dry-run` against a real board prints planned actions without mutations
