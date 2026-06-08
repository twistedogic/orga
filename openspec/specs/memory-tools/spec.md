# memory-tools Specification

## Purpose
Four agent tools (memory_list, memory_read, memory_write, memory_search) exposed by default to all main agents and subagents for reading and writing the context repository.

## Requirements

### Requirement: memory_list tool
The agent loop SHALL expose a `memory_list()` tool that returns the context repository file tree: all `.md` file paths with their frontmatter `description` fields. This tool SHALL be available to all main agents and subagents by default.

#### Scenario: Repository has files
- **WHEN** the agent calls `memory_list()`
- **THEN** a structured list of all `.md` file paths and descriptions is returned

#### Scenario: Empty repository
- **WHEN** the agent calls `memory_list()` and no files exist
- **THEN** an empty list is returned

### Requirement: memory_read tool
The agent loop SHALL expose a `memory_read(path)` tool that returns the full content of a specific file in the context repository. This tool SHALL be available to all main agents and subagents by default.

#### Scenario: File exists
- **WHEN** the agent calls `memory_read("themes/auth-complexity.md")`
- **THEN** the full content of that file is returned

#### Scenario: File does not exist
- **WHEN** the agent calls `memory_read` with a path that does not exist
- **THEN** an error string is returned indicating the file was not found

### Requirement: memory_write tool
The agent loop SHALL expose a `memory_write(path, content, commit_msg)` tool that writes (creates or overwrites) a file at the given path within the context repository and commits the change. This tool SHALL be available to all main agents and subagents by default.

#### Scenario: Create new file
- **WHEN** the agent calls `memory_write` with a path that does not exist
- **THEN** the file is created with the given content and a git commit is made

#### Scenario: Overwrite existing file
- **WHEN** the agent calls `memory_write` with a path that already exists
- **THEN** the file is overwritten and a new git commit is made

#### Scenario: Nested path auto-creates directories
- **WHEN** the agent calls `memory_write("themes/auth-complexity.md", ...)` and `themes/` does not exist
- **THEN** the `themes/` directory is created automatically

### Requirement: memory_search tool
The agent loop SHALL expose a `memory_search(query)` tool that performs a case-insensitive literal grep across all `.md` files in the context repository and returns matching lines with their file paths and line numbers. This tool SHALL be available to all main agents and subagents by default.

#### Scenario: Query matches content
- **WHEN** the agent calls `memory_search("JWT")`
- **THEN** all lines containing "JWT" (case-insensitive) are returned with their file paths and line numbers

#### Scenario: No matches
- **WHEN** the agent calls `memory_search` with a query that matches nothing
- **THEN** an empty result is returned

### Requirement: Memory tools available to all agents by default
The four memory tools (`memory_list`, `memory_read`, `memory_write`, `memory_search`) SHALL be included in the default tool set for both main agents and subagents, regardless of subagent configuration.

#### Scenario: Main agent has memory tools
- **WHEN** the main agent loop runs
- **THEN** all four memory tools are present in tool definitions

#### Scenario: Subagent has memory tools
- **WHEN** a subagent loop runs
- **THEN** all four memory tools are present in tool definitions
