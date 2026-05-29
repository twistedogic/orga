## Why

Agents working on long-running tasks need a place to read and write files during execution — storing intermediate data, reading input files, or producing output. Today there is no way for an agent to interact with the local filesystem; the only persistence mechanism is the artifact store, which is git-backed and commit-oriented, not suited for working-area file I/O.

## What Changes

- New `[workspace]` config section with a `path` key defining the workspace base directory
- New `WorkspaceStore` struct providing path-safe file access rooted at `<base>/<ticket_id>/`
- Three new agent tools: `read_file`, `write_file`, `list_files`
- `ToolContext` gains an optional `workspace` field (disabled if `[workspace]` not configured)
- All three tools added to `all_tool_definitions()` and the `dispatch` router

## Capabilities

### New Capabilities

- `agent-workspace`: Per-ticket workspace directory for agent file I/O — read, write, and list files within a sandboxed path rooted to the ticket's workspace

### Modified Capabilities

- `config`: New `[workspace]` section added with optional `path` field
- `agent-tools`: Three new file tools registered in the tool dispatcher

## Impact

- `src/config.rs` — add `WorkspaceConfig` struct and `workspace` field on `AppConfig`
- `src/agent/tools.rs` — add dispatch arms and tool definitions for `read_file`, `write_file`, `list_files`; add `workspace` to `ToolContext`
- New module `src/workspace.rs` — `WorkspaceStore` with path traversal protection
- No breaking changes; workspace tools are a no-op if `[workspace]` is not configured
