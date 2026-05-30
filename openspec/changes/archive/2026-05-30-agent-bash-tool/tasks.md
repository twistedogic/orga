## 1. Core Implementation

- [x] 1.1 Add `BashArgs` struct and `dispatch_bash` async fn to `src/agent/tools.rs` — runs `sh -c <command>` with `cwd` set to `ticket_root`, creates dir if missing, enforces 120s timeout via `tokio::time::timeout`, returns JSON `{ stdout, stderr, exit_code }`
- [x] 1.2 Add `"bash"` arm to the `dispatch` match in `src/agent/tools.rs`
- [x] 1.3 Add `bash` `ToolDefinition` to `all_tool_definitions()` in `src/agent/tools.rs`

## 2. Spec Updates

- [x] 2.1 Archive the `agent-tools` spec delta (update `openspec/specs/agent-tools/spec.md` to include `bash` in the file tools table and scenario)
- [x] 2.2 Archive the new `agent-bash-tool` spec to `openspec/specs/agent-bash-tool/spec.md`

## 3. Tests

- [x] 3.1 Add unit tests for `dispatch_bash`
