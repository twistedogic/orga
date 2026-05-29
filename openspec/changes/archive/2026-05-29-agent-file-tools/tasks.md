## 1. Config

- [x] 1.1 Add `WorkspaceConfig { path: String }` struct to `src/config.rs`
- [x] 1.2 Add optional `workspace: Option<WorkspaceConfig>` field to `AppConfig`
- [x] 1.3 Add `workspace_base_path()` helper on `AppConfig` that expands `~` and returns `PathBuf`

## 2. WorkspaceStore

- [x] 2.1 Create `src/workspace.rs` with `WorkspaceStore { base: PathBuf }`
- [x] 2.2 Implement ticket ID sanitization (replace `/`, `:`, and other unsafe chars with `_`)
- [x] 2.3 Implement `ticket_root(ticket_id) -> PathBuf` returning `<base>/<sanitized_id>/`
- [x] 2.4 Implement path safety check: join path to ticket root, verify resolved path starts with ticket root (handle pre-existence for write)
- [x] 2.5 Implement `read(ticket_id, path) -> Result<String, OrgaError>` — reads UTF-8 text, errors on binary or missing
- [x] 2.6 Implement `write(ticket_id, path, content) -> Result<(), OrgaError>` — creates dirs, writes text, enforces path safety
- [x] 2.7 Implement `list(ticket_id) -> Result<String, OrgaError>` — recursive walk, flat relative paths, newline-separated
- [x] 2.8 Export `WorkspaceStore` from `src/lib.rs`

## 3. Tool Wiring

- [x] 3.1 Add `workspace: Option<WorkspaceStore>` to `ToolContext` in `src/agent/tools.rs`
- [x] 3.2 Add `ReadFileArgs`, `WriteFileArgs` structs and dispatch functions `dispatch_read_file`, `dispatch_write_file`, `dispatch_list_files`
- [x] 3.3 Add dispatch arms for `"read_file"`, `"write_file"`, `"list_files"` in `dispatch()`
- [x] 3.4 Add `ToolDefinition` entries for `read_file`, `write_file`, `list_files` to `all_tool_definitions()`
- [x] 3.5 Mark `write_file` as mutating (apply dry-run guard consistent with other mutating tools)

## 4. Integration

- [x] 4.1 Build `WorkspaceStore` from config in agent loop setup (where `ToolContext` is constructed) and populate `workspace` field
- [x] 4.2 Verify `cargo build` passes with no warnings
- [x] 4.3 Verify `cargo clippy` passes clean
