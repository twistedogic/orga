## ADDED Requirements

### Requirement: ticket show includes sub_tickets field
`orga ticket show --json <id>` SHALL include a `sub_tickets` field containing an array of sub-ticket summaries. Each entry SHALL include: `id`, `title`, `url`, `list_name`, `completed`. On backends that do not support native parent-child relations (Trello), `sub_tickets` SHALL be an empty array.

#### Scenario: Ticket with sub-tickets (Linear)
- **WHEN** a Linear ticket has one or more child issues
- **THEN** `ticket show --json` returns a JSON object with `sub_tickets` as an array of objects, each with `id`, `title`, `url`, `list_name`, `completed`

#### Scenario: Ticket with no sub-tickets
- **WHEN** a ticket has no child issues
- **THEN** `ticket show --json` returns `sub_tickets: []`

#### Scenario: Trello backend always returns empty sub_tickets
- **WHEN** `ticket show --json` is called on a Trello backend
- **THEN** `sub_tickets` is always an empty array

#### Scenario: sub_tickets field present in all JSON responses
- **WHEN** `ticket show --json` is called for any ticket on any backend
- **THEN** the response JSON always includes the `sub_tickets` key (never absent)
