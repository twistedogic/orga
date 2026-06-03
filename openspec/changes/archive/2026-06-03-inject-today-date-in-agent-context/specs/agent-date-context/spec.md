## ADDED Requirements

### Requirement: Today's date in agent context
The system SHALL include today's date in the user message of every agent prompt, formatted as `**Today's date:** YYYY-MM-DD` using the local system date at invocation time.

#### Scenario: Main agent receives today's date
- **WHEN** the main agent loop builds its context
- **THEN** the user message SHALL contain a `**Today's date:**` field with the current local date in ISO 8601 format

#### Scenario: Subagent receives today's date
- **WHEN** a subagent loop builds its context
- **THEN** the user message SHALL contain a `**Today's date:**` field with the current local date in ISO 8601 format

#### Scenario: Date format is ISO 8601
- **WHEN** the date field is rendered
- **THEN** it SHALL use the format `YYYY-MM-DD` (e.g., `2026-06-03`)
