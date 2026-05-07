# artifact-cli Specification

## Purpose
Defines the `orga artifact` subcommand group: `commit`, `list`, and `get` — allowing agents to store, retrieve, and enumerate named text artifacts scoped per ticket and agent.

## Requirements

### Requirement: artifact commit subcommand
The CLI SHALL provide `orga artifact commit <ticket-id> <name> [content]` to commit a named text artifact for a ticket. Content SHALL be accepted either as a positional argument or via `--file <path>`. Exactly one of content or `--file` SHALL be provided.

#### Scenario: Inline content committed
- **WHEN** `orga artifact commit TICKET-123 report.md "my report text"`
- **THEN** the artifact is committed and human-readable confirmation is printed

#### Scenario: File content committed
- **WHEN** `orga artifact commit TICKET-123 report.md --file /path/to/report.md`
- **THEN** the file is read and its content is committed as the artifact

#### Scenario: Neither content nor --file provided
- **WHEN** `orga artifact commit TICKET-123 report.md` with no content or --file
- **THEN** the CLI exits with a non-zero code and an error message

#### Scenario: Both content and --file provided
- **WHEN** both a positional content argument and `--file` are given
- **THEN** the CLI exits with a non-zero code and an error message

#### Scenario: JSON output on success
- **WHEN** `--json` flag is set and commit succeeds
- **THEN** stdout contains `{"ok": true, "ticket_id": "...", "agent_name": "...", "name": "...", "committed_at": "..."}`

### Requirement: artifact list subcommand
The CLI SHALL provide `orga artifact list <ticket-id>` to list all artifacts for a ticket across all agents.

#### Scenario: Human-readable list
- **WHEN** `orga artifact list TICKET-123` and artifacts exist
- **THEN** each artifact is printed as `<agent-name>/<name>\t<committed_at>`

#### Scenario: Empty list
- **WHEN** no artifacts exist for the ticket
- **THEN** no output is produced (exit 0)

#### Scenario: JSON output
- **WHEN** `--json` flag is set
- **THEN** stdout contains a JSON array of ArtifactMeta objects

### Requirement: artifact get subcommand
The CLI SHALL provide `orga artifact get <ticket-id> <name>` to retrieve the current agent's artifact by name.

#### Scenario: Artifact found — human output
- **WHEN** `orga artifact get TICKET-123 report.md` and the artifact exists
- **THEN** the artifact content is printed to stdout

#### Scenario: Artifact not found
- **WHEN** the artifact does not exist for the current agent
- **THEN** the CLI exits with a non-zero code and an error message

#### Scenario: JSON output
- **WHEN** `--json` flag is set and artifact exists
- **THEN** stdout contains a JSON object with all `Artifact` fields including `content`
