## ADDED Requirements

### Requirement: Create sub-ticket with optional description and list
The CLI SHALL provide `orga ticket create-sub <parent_id> "<title>" [--description "<text>"] [--list "<column name>"]` to create a sub-ticket linked to a parent. The sub-ticket SHALL be created unassigned. If `--list` is omitted, the sub-ticket SHALL be placed in the same list as the parent. If `--list` is provided, the sub-ticket SHALL be placed in the named list, and the CLI SHALL error if no list with that name exists.

#### Scenario: Create sub-ticket with title only
- **WHEN** `orga ticket create-sub <parent_id> "<title>"` is run
- **THEN** a sub-ticket is created linked to the parent, unassigned, in the parent's current list

#### Scenario: Create sub-ticket with description
- **WHEN** `--description "<text>"` is provided
- **THEN** the sub-ticket is created with the given description text

#### Scenario: Create sub-ticket with explicit list
- **WHEN** `--list "<column name>"` is provided and that column exists
- **THEN** the sub-ticket is placed in that list instead of the parent's list

#### Scenario: List not found
- **WHEN** `--list "<column name>"` is provided and no column with that name exists
- **THEN** the CLI exits non-zero with an error message naming the missing list

#### Scenario: JSON output
- **WHEN** `--json` flag is passed
- **THEN** output is a JSON object with fields: `id`, `title`, `url`
