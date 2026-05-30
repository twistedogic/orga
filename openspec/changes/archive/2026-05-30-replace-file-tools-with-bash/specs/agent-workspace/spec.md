## REMOVED Requirements

### Requirement: read_file tool
**Reason**: Redundant with `bash` tool — agents can use `cat <path>` to read files from the workspace.
**Migration**: Replace `read_file { path: "notes.md" }` with `bash { command: "cat notes.md" }`.

### Requirement: write_file tool
**Reason**: Redundant with `bash` tool — agents can use `tee`, `echo >`, or `printf >` to write files.
**Migration**: Replace `write_file { path: "out.md", content: "..." }` with `bash { command: "printf '%s' '...' > out.md" }` or equivalent.

### Requirement: list_files tool
**Reason**: Redundant with `bash` tool — agents can use `find . -type f` or `ls -R` to list workspace files.
**Migration**: Replace `list_files {}` with `bash { command: "find . -type f | sort" }`.

### Requirement: Workspace not configured (file tools)
**Reason**: Removed along with the three file tools. The workspace-not-configured guard for `bash` remains.
**Migration**: None — `bash` already returns `error: workspace not configured` when `[workspace]` is absent.

## MODIFIED Requirements

### Requirement: Per-ticket workspace directory
The workspace SHALL provide each ticket with an isolated directory on the local filesystem at `<workspace.path>/<sanitized_ticket_id>/`, where `sanitized_ticket_id` replaces `/`, `:`, and other filesystem-unsafe characters with `_`. The workspace directory SHALL be created on first use by the `bash` tool.

#### Scenario: Workspace directory created on first bash invocation
- **WHEN** `bash` is called for a ticket whose workspace directory does not yet exist
- **THEN** the directory `<workspace.path>/<sanitized_ticket_id>/` is created before the command runs

#### Scenario: Ticket IDs with unsafe characters are sanitized
- **WHEN** a ticket has ID `PROJ-123/sub:task`
- **THEN** the workspace directory is resolved to `<workspace.path>/PROJ-123_sub_task/`
