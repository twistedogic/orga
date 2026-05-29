## 1. Config

- [x] 1.1 Add `SubagentConfig` struct to `src/config.rs` with fields: `name`, `description`, `tools`, `skills` (optional), `model` (optional), `max_actions` (optional)
- [x] 1.2 Add `subagents: Vec<SubagentConfig>` field (with `#[serde(default)]`) to `AppConfig`
- [x] 1.3 Add config validation: reject duplicate subagent names
- [x] 1.4 Add config validation: reject unknown tool names in subagent `tools` lists

## 2. Tool infrastructure

- [x] 2.1 Add `dispatch(subagent, task)` tool definition to `tool_definitions()` in `src/agent/tools.rs`
- [x] 2.2 Add `return(result)` tool definition to `tool_definitions()` (or a separate subagent-specific set)
- [x] 2.3 Add `is_terminal_tool` recognition for `return`
- [x] 2.4 Extract a `tool_definitions_for(names: &[String])` helper that returns only the named tools — used to build per-agent tool sets
- [x] 2.5 Add `dispatch_return` handler that captures the result string and signals loop termination
- [x] 2.6 Add `dispatch_dispatch` stub in `src/agent/tools.rs` (actual subagent invocation wired in step 4)

## 3. Subagent context builder

- [x] 3.1 Add `build_subagent_system_prompt(subagent_cfg, ticket, task, skill_ctx)` in `src/agent/context.rs`
- [x] 3.2 System prompt should identify the subagent by name, describe its role, list its tools, inject task string, and include skill bodies

## 4. Subagent loop

- [x] 4.1 Extract `run_subagent_loop<C>(client, subagent_cfg, ticket, task, dry_run, config, logger)` in `src/agent/mod.rs` — own LLM loop, own history, returns `String` (result)
- [x] 4.2 Subagent loop uses `tool_definitions_for(subagent_cfg.tools)` + `return`
- [x] 4.3 Subagent loop terminates on `return`, no-tool-call, or `max_actions` cap
- [x] 4.4 On cap-without-return, return synthetic error string to caller
- [x] 4.5 On no-tool-call, return last LLM text response as result
- [x] 4.6 Wire subagent skill injection: if `subagent_cfg.skills` is set, load those skills by name explicitly; otherwise fall back to `match_skills` on ticket title
- [x] 4.7 Wire `dispatch_dispatch` in `tools.rs` to call `run_subagent_loop` (pass subagent config lookup + client handle)

## 5. Main agent restructuring

- [x] 5.1 In `process_ticket`, detect whether subagents are configured (`config.subagents.is_empty()`)
- [x] 5.2 If subagents configured: use narrowed main agent tool set (`comment`, `dispatch`, `skip`, `done`)
- [x] 5.3 If subagents configured: inject subagent descriptions into main agent system prompt (new section in `build_system_prompt`)
- [x] 5.4 If no subagents: keep existing flat loop behavior unchanged

## 6. Dry-run support

- [x] 6.1 In dry-run mode, `dispatch` tool logs the call and returns a dry-run notice without starting the subagent loop

## 7. Tests

- [x] 7.1 Unit test: `SubagentConfig` deserialization from TOML (valid and invalid cases)
- [x] 7.2 Unit test: config validation rejects duplicate subagent names
- [x] 7.3 Unit test: config validation rejects unknown tool names in subagent tools list
- [x] 7.4 Unit test: `tool_definitions_for` returns correct subset
- [x] 7.5 Integration test: agent with subagents configured routes `dispatch` to subagent loop and returns result
- [x] 7.6 Integration test: agent without subagents configured uses full flat loop (regression)
