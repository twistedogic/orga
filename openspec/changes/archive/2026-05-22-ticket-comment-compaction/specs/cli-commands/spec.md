## ADDED Requirements

### Requirement: Ticket compact command
The CLI SHALL provide `orga ticket compact <id> --summary <text>` to store a compaction record for a ticket. The `--summary` argument SHALL be required and non-empty. On success the command SHALL print `{"ok": true}` with `--json` or a human-readable confirmation without it.

#### Scenario: Compact with summary
- **WHEN** `ticket compact <id> --summary "..."` is called with a non-empty summary
- **THEN** the compaction record is stored and the command exits with code 0

#### Scenario: Empty summary rejected
- **WHEN** `ticket compact <id> --summary ""` is called
- **THEN** the command exits with a non-zero code and prints an error

#### Scenario: JSON success output
- **WHEN** `ticket compact <id> --summary "..."` succeeds with `--json`
- **THEN** output is `{"ok": true}`

### Requirement: Ticket decompact command
The CLI SHALL provide `orga ticket decompact <id>` to delete the stored compaction record for a ticket. On success the command SHALL print `{"ok": true}` with `--json` or a human-readable confirmation without it.

#### Scenario: Decompact existing record
- **WHEN** `ticket decompact <id>` is called and a compaction record exists
- **THEN** the record is deleted and the command exits with code 0

#### Scenario: Decompact with no record is a no-op
- **WHEN** `ticket decompact <id>` is called and no compaction record exists
- **THEN** the command exits with code 0 without error

#### Scenario: JSON success output
- **WHEN** `ticket decompact <id>` succeeds with `--json`
- **THEN** output is `{"ok": true}`
