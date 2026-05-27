## MODIFIED Requirements

### Requirement: Ticket show command
The CLI SHALL provide `orga ticket show <id>` to output the full context of a ticket: title, description, current list, creator, assignees, sub-tickets, and all comments in chronological order. If the ticket's current column matches a `[[workflow]]` entry in config, the resolved prompt text SHALL be included in the output.

#### Scenario: Ticket exists
- **WHEN** a valid ticket ID is provided
- **THEN** full ticket context is printed including creator (if known), all comments and sub-tickets

#### Scenario: JSON output
- **WHEN** `--json` flag is passed
- **THEN** output is a JSON object with fields: `id`, `title`, `description`, `list`, `creator`, `assignees`, `sub_tickets`, `comments`

#### Scenario: JSON output with workflow prompt
- **WHEN** `--json` flag is passed and the ticket's column has a matching workflow entry
- **THEN** the JSON object additionally includes `workflow_prompt` containing the resolved prompt text

#### Scenario: JSON output without workflow prompt
- **WHEN** `--json` flag is passed and the ticket's column has no matching workflow entry
- **THEN** the JSON object does NOT include a `workflow_prompt` field

## REMOVED Requirements

### Requirement: Checklist commands
**Reason**: Checklists are replaced by sub-tickets. Linear's checklist items were already sub-issues under the hood. Trello checklist items had no structured identity agents could act on.
**Migration**: Use `orga ticket create-sub` to decompose work. Use `orga ticket show <sub_id>` to inspect a sub-ticket. Use `orga ticket move <sub_id> "<done column>"` to mark work complete.
