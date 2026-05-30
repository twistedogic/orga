## 1. Remove file tools from src/agent/tools.rs

- [x] 1.1 Remove dispatch arms for `"read_file"`, `"write_file"`, `"list_files"` from the `dispatch()` match
- [x] 1.2 Remove `ReadFileArgs` struct and `dispatch_read_file` function
- [x] 1.3 Remove `WriteFileArgs` struct and `dispatch_write_file` function
- [x] 1.4 Remove `dispatch_list_files` function
- [x] 1.5 Remove `ToolDefinition` entries for `read_file`, `write_file`, `list_files` from `all_tool_definitions()`
- [x] 1.6 Remove tests covering the three removed tools

## 2. Update config validation in src/config.rs

- [x] 2.1 Remove `"read_file"`, `"write_file"`, `"list_files"` from `VALID_TOOLS`
- [x] 2.2 Add `"bash"` to `VALID_TOOLS`

## 3. Update agent-workspace spec

- [x] 3.1 Rewrite `openspec/specs/agent-workspace/spec.md` — remove `read_file`, `write_file`, `list_files` requirements and update workspace directory creation scenario to reference `bash`

## 4. Verify

- [x] 4.1 Run `cargo test` and confirm all tests pass
- [x] 4.2 Run `cargo clippy` and confirm no warnings
