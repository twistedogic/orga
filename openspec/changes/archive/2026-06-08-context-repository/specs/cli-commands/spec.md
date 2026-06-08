## REMOVED Requirements

### Requirement: Memory set command
**Reason**: `orga memory set <ticket-id>` is replaced by `orga memory write <path>` — topic-path writes replace ticket-ID blob writes. See `agent-memory` capability delta.
**Migration**: Use `orga memory write <path> "<content>"`.

### Requirement: Memory get command
**Reason**: `orga memory get <ticket-id>` is replaced by `orga memory list` and `orga memory read <path>`. See `agent-memory` capability delta.
**Migration**: Use `orga memory list` then `orga memory read <path>`.

## ADDED Requirements

### Requirement: memory list CLI command
The CLI SHALL provide `orga memory list` as described in the `agent-memory` capability spec.

#### Scenario: memory list in command reference
- **WHEN** `orga memory list --json` is called
- **THEN** a JSON array of topic file entries is returned

### Requirement: memory read CLI command
The CLI SHALL provide `orga memory read <path>` as described in the `agent-memory` capability spec.

#### Scenario: memory read in command reference
- **WHEN** `orga memory read <path>` is called with a valid path
- **THEN** the file content is printed to stdout

### Requirement: memory write CLI command
The CLI SHALL provide `orga memory write <path> <content>` as described in the `agent-memory` capability spec.

#### Scenario: memory write in command reference
- **WHEN** `orga memory write <path> "<content>"` is called
- **THEN** the file is written and committed

### Requirement: memory search CLI command
The CLI SHALL provide `orga memory search <query>` as described in the `agent-memory` capability spec.

#### Scenario: memory search in command reference
- **WHEN** `orga memory search "<query>"` is called
- **THEN** matching lines are returned with file path and line number

### Requirement: memory defrag CLI command
The CLI SHALL provide `orga memory defrag` as described in the `sleep-time-agent` capability spec.

#### Scenario: memory defrag in command reference
- **WHEN** `orga memory defrag` is called
- **THEN** a defragmentation pass runs and commits the result
