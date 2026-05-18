## MODIFIED Requirements

### Requirement: Board trait definition
The system SHALL define a `Board` trait that all backend adapters must implement. The trait SHALL be the only interface the CLI uses to interact with a board — no backend-specific code SHALL appear in CLI command handlers.

#### Scenario: Backend resolution
- **WHEN** the CLI starts and reads the config
- **THEN** the correct backend implementation is instantiated based on `board.backend` config value

#### Scenario: Unknown backend
- **WHEN** the config specifies an unrecognized backend name
- **THEN** the CLI exits with a non-zero code and prints an error listing supported backends: `trello`, `linear`
