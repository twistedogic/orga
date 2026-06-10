# agent-memory Specification

## Purpose
Topic-organized, git-backed context repository for agent memory. Memory is organized by topic (themes, patterns, people, architecture) rather than per-ticket, enabling cross-ticket recall and cumulative learning.

## Requirements

### Requirement: Topic-organized context repository
The system SHALL provide a local context repository organized as topic-named markdown files in a git-backed directory. The repository SHALL persist across CLI invocations. The default path SHALL be `~/.orga/memory/`, overridable via `[memory] path` in config. The implementation SHALL live in `src/memory/context_repo.rs` and be re-exported from `src/memory/mod.rs` as `crate::memory::ContextRepository`.

#### Scenario: Repository persists between invocations
- **WHEN** the sleep-time agent writes a topic file and the CLI exits
- **THEN** a subsequent CLI invocation can read the same file via `orga memory read`

#### Scenario: Import path unchanged
- **WHEN** any module imports `crate::memory::ContextRepository`
- **THEN** the import resolves correctly after the module split

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

### Requirement: Compaction store module
`CompactionStore` SHALL live in `src/memory/compaction.rs` and be re-exported from `src/memory/mod.rs` as `crate::memory::CompactionStore`.

#### Scenario: Import path unchanged after split
- **WHEN** any module imports `crate::memory::CompactionStore`
- **THEN** the import resolves correctly

### Requirement: Todo store module
`TodoStore` SHALL live in `src/memory/todo.rs` and be re-exported from `src/memory/mod.rs` as `crate::memory::TodoStore`.

#### Scenario: Import path unchanged after split
- **WHEN** any module imports `crate::memory::TodoStore`
- **THEN** the import resolves correctly

