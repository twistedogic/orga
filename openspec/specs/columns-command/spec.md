# columns-command Specification

## Requirements

### Requirement: Columns command
The CLI SHALL provide `orga columns` as a top-level command that lists all columns (lists) on the configured board. Output SHALL include each column's `id` and `name`. With `--json`, output SHALL be a JSON array of column objects.

#### Scenario: Board has columns
- **WHEN** the board has one or more columns
- **THEN** each column is printed with its `id` and `name`, one per line

#### Scenario: JSON output
- **WHEN** `--json` flag is passed
- **THEN** output is a valid JSON array of objects, each with fields `id` and `name`

#### Scenario: Backend error
- **WHEN** the board API returns an error
- **THEN** the command exits with a non-zero code and prints an error message to stderr
