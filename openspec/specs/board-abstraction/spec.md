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
The `Board` trait SHALL operate on a shared `Ticket` type that is backend-agnostic. The `Ticket` type SHALL include: `id`, `title`, `description`, `list_id`, `list_name`, `url`, `completed` (bool), `assignees` (Vec of usernames), `checklists` (Vec of checklist with items), and `comments` (Vec of Comment). The `completed` field SHALL be `true` when the ticket is closed/archived on the backend.

#### Scenario: Ticket serialization
- **WHEN** a ticket is returned from any backend
- **THEN** it can be serialized to JSON using the shared type without backend-specific fields leaking

#### Scenario: Completed ticket serialization
- **WHEN** a closed/archived ticket is returned from any backend
- **THEN** its JSON representation includes `"completed": true`

#### Scenario: Open ticket serialization
- **WHEN** an open ticket is returned from any backend
- **THEN** its JSON representation includes `"completed": false`

### Requirement: Error handling
The `Board` trait methods SHALL return `Result<T, OrgaError>` where `OrgaError` is a shared error type covering: not found, unauthorized, rate limited, network failure, and backend-specific errors (wrapped).

#### Scenario: Network failure
- **WHEN** a backend call fails due to a network error
- **THEN** the CLI prints a human-readable error to stderr and exits non-zero

#### Scenario: Not found error
- **WHEN** a ticket or list ID does not exist
- **THEN** the CLI prints "not found: <id>" to stderr and exits non-zero

### Requirement: Column data model
The `Board` trait SHALL operate on a shared `Column` type that is backend-agnostic. The `Column` type SHALL include: `id` (String) and `name` (String). It SHALL derive `Debug`, `Clone`, `Serialize`, and `Deserialize`.

#### Scenario: Column serialization
- **WHEN** a column is returned from any backend
- **THEN** it can be serialized to JSON with exactly the fields `id` and `name`, without backend-specific fields leaking

### Requirement: list_columns trait method
The `Board` trait SHALL define a `list_columns() -> Result<Vec<Column>, OrgaError>` method. All backend implementations SHALL implement this method.

#### Scenario: Columns returned
- **WHEN** `list_columns()` is called on a valid board
- **THEN** it returns a `Vec<Column>` with one entry per column on the board

#### Scenario: Backend failure
- **WHEN** the underlying API call fails
- **THEN** `list_columns()` returns an `Err(OrgaError)` with an appropriate variant

### Requirement: list_assigned returns all tickets
The `list_assigned` method on the `Board` trait SHALL return all tickets assigned to the agent, regardless of completion state. Filtering by completion state is the caller's responsibility.

#### Scenario: Mix of open and completed tickets
- **WHEN** the agent has both open and closed tickets assigned
- **THEN** `list_assigned` returns all of them with `completed` set correctly on each

