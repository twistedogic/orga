# board-abstraction Specification

## Purpose
TBD - created by archiving change agent-board-cli. Update Purpose after archive.
## Requirements
### Requirement: Board trait definition
The system SHALL define a `Board` trait that all backend adapters must implement. The trait SHALL be the only interface the CLI uses to interact with a board — no backend-specific code SHALL appear in CLI command handlers.

#### Scenario: Backend resolution
- **WHEN** the CLI starts and reads the config
- **THEN** the correct backend implementation is instantiated based on `board.backend` config value

#### Scenario: Unknown backend
- **WHEN** the config specifies an unrecognized backend name
- **THEN** the CLI exits with a non-zero code and prints an error listing supported backends

### Requirement: Ticket data model
The `Board` trait SHALL operate on a shared `Ticket` type that is backend-agnostic. The `Ticket` type SHALL include: `id`, `title`, `description`, `list_id`, `list_name`, `url`, `assignees` (Vec of usernames), `checklists` (Vec of checklist with items), and `comments` (Vec of Comment).

#### Scenario: Ticket serialization
- **WHEN** a ticket is returned from any backend
- **THEN** it can be serialized to JSON using the shared type without backend-specific fields leaking

### Requirement: Error handling
The `Board` trait methods SHALL return `Result<T, OrgaError>` where `OrgaError` is a shared error type covering: not found, unauthorized, rate limited, network failure, and backend-specific errors (wrapped).

#### Scenario: Network failure
- **WHEN** a backend call fails due to a network error
- **THEN** the CLI prints a human-readable error to stderr and exits non-zero

#### Scenario: Not found error
- **WHEN** a ticket or list ID does not exist
- **THEN** the CLI prints "not found: <id>" to stderr and exits non-zero

