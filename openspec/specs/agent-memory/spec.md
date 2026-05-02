# agent-memory Specification

## Purpose
TBD - created by archiving change agent-board-cli. Update Purpose after archive.
## Requirements
### Requirement: Per-ticket memory store
The system SHALL provide a local memory store keyed by ticket ID. The store SHALL persist across CLI invocations. The default path SHALL be `~/.orga/memory.db` (SQLite), overridable via config.

#### Scenario: Memory persists between invocations
- **WHEN** an agent sets memory for a ticket and the CLI exits
- **THEN** a subsequent invocation can retrieve the same memory for that ticket

### Requirement: Memory set command
The CLI SHALL provide `orga memory set <ticket-id> <context>` to store an arbitrary text string as the agent's working context for a ticket. Calling `set` again SHALL overwrite the previous value.

#### Scenario: Set memory
- **WHEN** `orga memory set ABC-1 "analyzed 3 files"` is called
- **THEN** the context is stored and a success message is printed

#### Scenario: Overwrite memory
- **WHEN** `orga memory set` is called for a ticket that already has memory
- **THEN** the previous value is replaced with the new value

### Requirement: Memory get command
The CLI SHALL provide `orga memory get <ticket-id>` to retrieve the stored context for a ticket. With `--json`, output SHALL be `{"ticket_id": "...", "context": "...", "updated_at": "..."}`.

#### Scenario: Memory exists
- **WHEN** memory has been set for a ticket
- **THEN** the context text is printed to stdout

#### Scenario: No memory set
- **WHEN** no memory has been set for the given ticket ID
- **THEN** the command exits with code 0 and prints nothing (or `{"ticket_id": "...", "context": null}` with `--json`)

### Requirement: Memory database initialization
The memory database SHALL be created automatically on first use if it does not exist. The parent directory SHALL also be created if missing.

#### Scenario: First run
- **WHEN** `orga memory set` is called and `~/.orga/memory.db` does not exist
- **THEN** the directory and database file are created and the memory is stored successfully

