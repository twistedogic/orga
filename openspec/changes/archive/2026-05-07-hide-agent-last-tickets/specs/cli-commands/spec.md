## MODIFIED Requirements

### Requirement: Ticket list command
The CLI SHALL provide `orga ticket list` to output all tickets currently assigned to the configured agent that are not completed AND where the latest comment was not posted by an agent. With `--completed`, output SHALL include only completed tickets. With `--all`, output SHALL include all tickets regardless of completion state or latest commenter. Output SHALL include ticket ID, title, list name, and URL. With `--json`, output SHALL be a JSON array of ticket objects.

#### Scenario: Tickets assigned to agent (default)
- **WHEN** the agent has one or more open assigned tickets where the latest comment is not from an agent
- **THEN** each such ticket is printed with its ID, title, current list, and URL

#### Scenario: Agent-last tickets hidden by default
- **WHEN** an assigned open ticket's latest comment was posted by an agent
- **THEN** that ticket SHALL NOT appear in the default `ticket list` output

#### Scenario: No open tickets needing response
- **WHEN** all assigned open tickets have their latest comment from an agent (or there are no assigned open tickets)
- **THEN** the command exits with code 0 and prints nothing (or empty JSON array with `--json`)

#### Scenario: JSON output
- **WHEN** `--json` flag is passed
- **THEN** output is a valid JSON array of ticket objects with fields: `id`, `title`, `list`, `url`, `description`, `completed`, `last_commenter_is_agent`

#### Scenario: Completed tickets only
- **WHEN** `--completed` flag is passed
- **THEN** only tickets with `completed: true` are printed (agent-last filter does not apply)

#### Scenario: All tickets
- **WHEN** `--all` flag is passed
- **THEN** all tickets assigned to the agent are printed regardless of completion state or latest commenter

#### Scenario: `--completed` and `--all` are mutually exclusive
- **WHEN** both `--completed` and `--all` are passed
- **THEN** the command exits with a non-zero code and prints a usage error
