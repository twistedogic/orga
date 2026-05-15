## MODIFIED Requirements

### Requirement: Ticket show command
The CLI SHALL provide `orga ticket show <id>` to output the full context of a ticket: title, description, current list, creator, assignees, checklist items, and all comments in chronological order. If the ticket's current column matches a `[[workflow]]` entry in config, the resolved prompt text SHALL be included in the output.

#### Scenario: Ticket exists
- **WHEN** a valid ticket ID is provided
- **THEN** full ticket context is printed including creator (if known), all comments and checklist items

#### Scenario: JSON output
- **WHEN** `--json` flag is passed
- **THEN** output is a JSON object with fields: `id`, `title`, `description`, `list`, `creator`, `assignees`, `checklists`, `comments`

#### Scenario: JSON output with workflow prompt
- **WHEN** `--json` flag is passed and the ticket's column has a matching workflow entry
- **THEN** the JSON object additionally includes `workflow_prompt` containing the resolved prompt text

#### Scenario: JSON output without workflow prompt
- **WHEN** `--json` flag is passed and the ticket's column has no matching workflow entry
- **THEN** the JSON object does NOT include a `workflow_prompt` field

#### Scenario: Human-readable output with workflow prompt
- **WHEN** no `--json` flag and the ticket's column has a matching workflow entry
- **THEN** the output includes a `## Workflow` section at the end containing the prompt text

#### Scenario: Human-readable output without workflow prompt
- **WHEN** no `--json` flag and the ticket's column has no matching workflow entry
- **THEN** no `## Workflow` section appears in the output

#### Scenario: Ticket not found
- **WHEN** an invalid or nonexistent ticket ID is provided
- **THEN** the command exits with a non-zero code and prints an error message to stderr
