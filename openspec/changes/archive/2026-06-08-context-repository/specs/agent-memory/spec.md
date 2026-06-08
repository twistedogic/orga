## REMOVED Requirements

### Requirement: Per-ticket memory store
**Reason**: Replaced by the topic-organized context repository (`context-repository` capability). Per-ticket SQLite memory does not support cross-ticket recall, provides no structure for theme/pattern organization, and is opaque to human inspection.
**Migration**: No automated migration. Existing `~/.orga/memory.db` is abandoned. The new context repository is initialized fresh at `~/.orga/memory/`. Agents will build up topic knowledge organically through the sleep-time agent after each `done()`.

### Requirement: Memory set command
**Reason**: Replaced by `orga memory write <path> <content>` — topic-path-based writes replace ticket-ID-based blob writes.
**Migration**: Use `orga memory write <topic-path> "<content>"` to write topic files directly.

### Requirement: Memory get command
**Reason**: Replaced by `orga memory read <path>` and `orga memory list` — topic-path reads replace ticket-ID blob reads.
**Migration**: Use `orga memory list` to see available topics, then `orga memory read <path>` to read a specific file.

### Requirement: Memory database initialization
**Reason**: Replaced by git repository initialization in the `context-repository` capability.
**Migration**: The context repository is initialized automatically at `~/.orga/memory/` on first use.

## ADDED Requirements

### Requirement: Topic-organized context repository
The system SHALL provide a local context repository organized as topic-named markdown files in a git-backed directory. The repository SHALL persist across CLI invocations. The default path SHALL be `~/.orga/memory/`, overridable via `[memory] path` in config.

#### Scenario: Repository persists between invocations
- **WHEN** the sleep-time agent writes a topic file and the CLI exits
- **THEN** a subsequent CLI invocation can read the same file via `orga memory read`

### Requirement: memory list command
The CLI SHALL provide `orga memory list` to output the context repository file tree: all `.md` file paths with their frontmatter `description` fields. With `--json`, output SHALL be a JSON array of objects with `path` and `description` fields.

#### Scenario: Repository has files
- **WHEN** `orga memory list` is called and the repository has files
- **THEN** each file path and description is printed

#### Scenario: Empty repository
- **WHEN** `orga memory list` is called and the repository is empty
- **THEN** the command exits with code 0 and prints nothing (or empty JSON array with `--json`)

#### Scenario: JSON output
- **WHEN** `orga memory list --json` is called
- **THEN** output is a valid JSON array of `{"path": "...", "description": "..."}` objects

### Requirement: memory read command
The CLI SHALL provide `orga memory read <path>` to output the full content of a topic file. With `--json`, output SHALL be `{"path": "...", "content": "..."}`.

#### Scenario: File exists
- **WHEN** `orga memory read themes/auth-complexity.md` is called and the file exists
- **THEN** the file content is printed to stdout

#### Scenario: File does not exist
- **WHEN** `orga memory read` is called with a non-existent path
- **THEN** the command exits with a non-zero code and prints an error

### Requirement: memory write command
The CLI SHALL provide `orga memory write <path> <content>` to write (create or overwrite) a topic file and commit the change. An optional `--message` flag sets the commit message (default: "write: <path>").

#### Scenario: Create new file
- **WHEN** `orga memory write themes/auth.md "..."` is called and the file does not exist
- **THEN** the file is created and committed

#### Scenario: Overwrite existing file
- **WHEN** `orga memory write` is called with an existing path
- **THEN** the file is overwritten and a new commit is made

### Requirement: memory search command
The CLI SHALL provide `orga memory search <query>` to perform a case-insensitive literal search across all `.md` files in the repository. Output SHALL include file path, line number, and matching line. With `--json`, output SHALL be a JSON array of `{"path": "...", "line": N, "content": "..."}` objects.

#### Scenario: Query matches
- **WHEN** `orga memory search "auth"` is called and matches exist
- **THEN** each matching line is printed with its file path and line number

#### Scenario: No matches
- **WHEN** `orga memory search` is called with a query that matches nothing
- **THEN** the command exits with code 0 and prints nothing

### Requirement: memory defrag command
The CLI SHALL provide `orga memory defrag` to manually trigger a defragmentation pass against the context repository.

#### Scenario: Manual defrag
- **WHEN** `orga memory defrag` is called
- **THEN** a defragmentation LLM pass runs and the result is committed to the repository
