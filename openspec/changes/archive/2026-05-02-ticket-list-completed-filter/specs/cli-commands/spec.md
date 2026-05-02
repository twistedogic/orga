## MODIFIED Requirements

### Requirement: Ticket list command
The CLI SHALL provide `orga ticket list` to output all tickets currently assigned to the configured agent that are not completed. With `--completed`, output SHALL include only completed tickets. With `--all`, output SHALL include all tickets regardless of completion state. Output SHALL include ticket ID, title, list name, and URL. With `--json`, output SHALL be a JSON array of ticket objects.

#### Scenario: Tickets assigned to agent (default)
- **WHEN** the agent has one or more open assigned tickets
- **THEN** each open ticket is printed with its ID, title, current list, and URL

#### Scenario: No open tickets assigned
- **WHEN** the agent has no open assigned tickets
- **THEN** the command exits with code 0 and prints nothing (or empty JSON array with `--json`)

#### Scenario: JSON output
- **WHEN** `--json` flag is passed
- **THEN** output is a valid JSON array of ticket objects with fields: `id`, `title`, `list`, `url`, `description`, `completed`

#### Scenario: Completed tickets only
- **WHEN** `--completed` flag is passed
- **THEN** only tickets with `completed: true` are printed

#### Scenario: All tickets
- **WHEN** `--all` flag is passed
- **THEN** all tickets assigned to the agent are printed regardless of completion state

#### Scenario: `--completed` and `--all` are mutually exclusive
- **WHEN** both `--completed` and `--all` are passed
- **THEN** the command exits with a non-zero code and prints a usage error
