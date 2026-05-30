# agent-workspace Specification

## Purpose
Per-ticket workspace directory for agent file I/O. Provides each ticket with an isolated local filesystem directory rooted at `<workspace.path>/<sanitized_ticket_id>/`. Agents interact with workspace files via the `bash` tool using standard shell utilities (`cat`, `echo`, `find`, etc.).

## Requirements

### Requirement: Per-ticket workspace directory
The workspace SHALL provide each ticket with an isolated directory on the local filesystem at `<workspace.path>/<sanitized_ticket_id>/`, where `sanitized_ticket_id` replaces `/`, `:`, and other filesystem-unsafe characters with `_`. The workspace directory SHALL be created on first use by the `bash` tool.

#### Scenario: Workspace directory created on first bash invocation
- **WHEN** `bash` is called for a ticket whose workspace directory does not yet exist
- **THEN** the directory `<workspace.path>/<sanitized_ticket_id>/` is created before the command runs

#### Scenario: Ticket IDs with unsafe characters are sanitized
- **WHEN** a ticket has ID `PROJ-123/sub:task`
- **THEN** the workspace directory is resolved to `<workspace.path>/PROJ-123_sub_task/`

### Requirement: bash tool workspace integration
The `bash` tool SHALL run shell commands with the ticket workspace directory as the working directory. Agents use standard shell utilities for file operations (`cat` to read, `tee`/`echo >` to write, `find`/`ls` to list).

#### Scenario: bash executes in workspace directory
- **WHEN** `bash { command: "pwd" }` is called
- **THEN** the output is the ticket's workspace path `<workspace.path>/<sanitized_ticket_id>/`

#### Scenario: Workspace not configured
- **WHEN** `[workspace]` is absent from the config and an agent calls `bash`
- **THEN** the tool returns `error: workspace not configured`
