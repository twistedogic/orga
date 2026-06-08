# memory-delete Specification

## Purpose
Provides `ContextRepository::delete()` with a frontmatter-based uniqueness guardrail and exposes `memory_delete` as a tool exclusively for the defrag agent.

## Requirements

### Requirement: ContextRepository delete with uniqueness guardrail
`ContextRepository::delete(path)` SHALL remove a `.md` file from the context repository and commit the deletion with message `delete: {path}`. Before deleting, it SHALL check whether the file's frontmatter `description` terms appear in any other `.md` file. If the file has a `description` and none of its terms appear elsewhere, deletion SHALL be blocked with an error. If the file has no frontmatter `description`, deletion SHALL be allowed unconditionally.

Significant terms are extracted by: splitting the description on whitespace and punctuation, lowercasing, keeping words ≥ 3 characters, and excluding the stopwords: `the`, `and`, `for`, `not`, `are`, `was`, `but`, `its`.

#### Scenario: Delete file with covered description terms
- **WHEN** `delete("themes/auth-notes.md")` is called and another file contains at least one of its description terms
- **THEN** the file is deleted and committed as `delete: themes/auth-notes.md`

#### Scenario: Delete blocked when all terms are unique
- **WHEN** `delete("themes/obscure.md")` is called and no other file contains any of its description terms
- **THEN** an error is returned: `cannot delete: no other file covers its topics`

#### Scenario: Delete file with no frontmatter
- **WHEN** `delete("notes.md")` is called and the file has no YAML frontmatter
- **THEN** the file is deleted and committed unconditionally

#### Scenario: Delete file with empty description
- **WHEN** `delete("stub.md")` is called and its frontmatter `description` is empty
- **THEN** the file is deleted and committed unconditionally

#### Scenario: Delete non-existent file returns error
- **WHEN** `delete("nonexistent.md")` is called and the file does not exist
- **THEN** an error is returned indicating the file was not found

### Requirement: memory_delete tool for defrag agent
The `memory_delete(path)` tool SHALL be available exclusively to the defrag agent via `SleepToolContext`. It SHALL NOT be included in `all_tool_definitions()` and SHALL NOT be available to main agents or subagents during ticket cycles.

#### Scenario: Defrag agent can call memory_delete
- **WHEN** the defrag agent calls `memory_delete(path: "themes/old-notes.md")`
- **THEN** `ContextRepository::delete()` is called and the result returned to the agent

#### Scenario: memory_delete not available to main agent
- **WHEN** the main agent loop runs
- **THEN** `memory_delete` is NOT present in the tool definitions

#### Scenario: memory_delete not available to subagent
- **WHEN** a subagent loop runs
- **THEN** `memory_delete` is NOT present in the tool definitions

### Requirement: orga memory delete CLI command
The CLI SHALL provide `orga memory delete <path>` to delete a topic file from the context repository. It SHALL apply the same uniqueness guardrail as `ContextRepository::delete()`. With `--json`, success output SHALL be `{"ok": true, "path": "..."}`.

#### Scenario: Delete succeeds
- **WHEN** `orga memory delete <path>` is called on a file whose description terms are covered elsewhere
- **THEN** the file is deleted, committed, and a success message is printed

#### Scenario: Delete blocked
- **WHEN** `orga memory delete <path>` is called on a file whose description terms appear nowhere else
- **THEN** the command exits with a non-zero code and prints the guardrail error

#### Scenario: JSON success output
- **WHEN** `orga memory delete <path>` succeeds with `--json`
- **THEN** output is `{"ok": true, "path": "<path>"}`
