## ADDED Requirements

### Requirement: Per-ticket workspace directory
The workspace SHALL provide each ticket with an isolated directory on the local filesystem at `<workspace.path>/<sanitized_ticket_id>/`, where `sanitized_ticket_id` replaces `/`, `:`, and other filesystem-unsafe characters with `_`. The workspace directory SHALL be created on first use.

#### Scenario: Workspace directory created on first write
- **WHEN** `write_file` is called for a ticket whose workspace directory does not yet exist
- **THEN** the directory `<workspace.path>/<sanitized_ticket_id>/` is created before the file is written

#### Scenario: Ticket IDs with unsafe characters are sanitized
- **WHEN** a ticket has ID `PROJ-123/sub:task`
- **THEN** the workspace directory is resolved to `<workspace.path>/PROJ-123_sub_task/`

### Requirement: Path traversal protection
All file operations SHALL resolve the requested path relative to the ticket workspace root and verify the resolved absolute path is within the ticket workspace directory. If the resolved path escapes the root, the operation SHALL return `error: path escapes workspace root`.

#### Scenario: Path traversal via `../` is rejected
- **WHEN** an agent calls `read_file` with path `../../etc/passwd`
- **THEN** the tool returns `error: path escapes workspace root`

#### Scenario: Nested path within workspace is allowed
- **WHEN** an agent calls `read_file` with path `output/results/data.csv`
- **THEN** the file is read from `<workspace>/<ticket_id>/output/results/data.csv`

### Requirement: read_file tool
The `read_file` tool SHALL read a UTF-8 text file from the ticket workspace and return its content as a string. If the file does not exist, it SHALL return `error: file not found`. If the file contains binary (non-UTF-8) content, it SHALL return `error: file contains binary content`.

#### Scenario: Successful text file read
- **WHEN** `read_file { path: "notes.md" }` is called and the file exists and is valid UTF-8
- **THEN** the full file content is returned as a string

#### Scenario: File not found
- **WHEN** `read_file { path: "missing.txt" }` is called and the file does not exist
- **THEN** the tool returns `error: file not found`

#### Scenario: Binary file rejected
- **WHEN** `read_file { path: "image.png" }` is called and the file contains non-UTF-8 bytes
- **THEN** the tool returns `error: file contains binary content`

### Requirement: write_file tool
The `write_file` tool SHALL write a string as the contents of a file in the ticket workspace, creating intermediate directories as needed. If the file already exists it SHALL be overwritten.

#### Scenario: Successful file write
- **WHEN** `write_file { path: "output/report.md", content: "# Report\n..." }` is called
- **THEN** the file is created at `<workspace>/<ticket_id>/output/report.md` with the given content

#### Scenario: Intermediate directories created automatically
- **WHEN** `write_file { path: "a/b/c/file.txt", content: "data" }` is called and directories do not exist
- **THEN** all intermediate directories are created and the file is written

#### Scenario: Existing file overwritten
- **WHEN** `write_file` is called for a path that already exists
- **THEN** the file is overwritten with the new content

### Requirement: list_files tool
The `list_files` tool SHALL return a newline-separated flat list of all file paths relative to the ticket workspace root, found by recursively walking the workspace directory. If the workspace directory does not yet exist or is empty, it SHALL return an empty string.

#### Scenario: Lists all files recursively
- **WHEN** the workspace contains `notes.md`, `output/report.md`, and `data/raw.csv`
- **THEN** `list_files` returns `data/raw.csv\nnotes.md\noutput/report.md` (or any order)

#### Scenario: Empty workspace returns empty string
- **WHEN** the ticket workspace directory does not exist or contains no files
- **THEN** `list_files` returns an empty string

### Requirement: Workspace not configured
If `[workspace]` is not present in the config, all three file tools SHALL return `error: workspace not configured`.

#### Scenario: Tools fail gracefully without config
- **WHEN** `[workspace]` is absent from the config and an agent calls `read_file`
- **THEN** the tool returns `error: workspace not configured`
