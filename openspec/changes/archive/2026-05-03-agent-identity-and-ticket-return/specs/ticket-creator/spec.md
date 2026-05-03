## ADDED Requirements

### Requirement: Ticket carries creator
The `Ticket` model SHALL include a `creator` field of type `Option<Member>`. On every `ticket show` invocation, the backend SHALL fetch the ticket's `createCard` action and populate `creator` with the `memberCreator` from that action. If no `createCard` action is available, `creator` SHALL be `None`.

#### Scenario: Creator present
- **WHEN** `ticket show <id>` is run and the ticket has a `createCard` action
- **THEN** the returned ticket includes `creator` with `id`, `username`, and `full_name`

#### Scenario: Creator absent (old card)
- **WHEN** `ticket show <id>` is run and no `createCard` action exists
- **THEN** `creator` is `null` in JSON output and omitted from human-readable output

#### Scenario: Creator in JSON output
- **WHEN** `ticket show <id> --json` is run
- **THEN** the JSON object includes a `creator` field (object or null)
