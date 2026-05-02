## ADDED Requirements

### Requirement: Columns command in CLI
The CLI SHALL provide `orga columns` as a top-level command (alongside `init`, `ticket`, `checklist`, `memory`). It SHALL appear in `orga --help` output with a brief description.

#### Scenario: Columns appears in help
- **WHEN** `orga --help` is run
- **THEN** `columns` is listed as an available command with a brief description

#### Scenario: Columns does not require a subcommand
- **WHEN** `orga columns` is run with no additional arguments
- **THEN** the command executes and outputs the list of columns
