## ADDED Requirements

### Requirement: Trello backend implements Board trait
The system SHALL provide a `TrelloBackend` struct implementing the `Board` trait using the Trello REST API v1. All operations SHALL use the `api_key` and `token` from config for authentication.

#### Scenario: Authenticated request
- **WHEN** any Trello API call is made
- **THEN** the request includes `key` and `token` query parameters from the config

### Requirement: List assigned tickets
`TrelloBackend::list_assigned` SHALL fetch all open cards assigned to the configured `trello_member_id` on the configured board.

#### Scenario: Filter by board
- **WHEN** listing assigned tickets
- **THEN** only cards belonging to the configured board ID are returned, not all cards across all boards

### Requirement: Sub-ticket linking
Since Trello has no native parent/child card relationship, `TrelloBackend::create_sub` SHALL create a new card on the same board and SHALL add a checklist item on the parent card with a link to the new card.

#### Scenario: Sub-ticket created
- **WHEN** `create_sub` is called with a parent card ID and title
- **THEN** a new card is created in the same list as the parent
- **THEN** a checklist named "Sub-tasks" is created (or reused) on the parent card with a link to the new card

### Requirement: Username resolution for assign
`TrelloBackend::assign` SHALL accept a Trello username (or `@username`) and resolve it to a member ID before calling the Trello add-member API.

#### Scenario: Valid username
- **WHEN** a valid Trello username is provided
- **THEN** the member is looked up and added to the card

#### Scenario: Invalid username
- **WHEN** the username does not match any Trello member
- **THEN** an error is returned

### Requirement: Rate limit handling
The Trello backend SHALL detect HTTP 429 responses and return a rate-limited `OrgaError` variant rather than crashing.

#### Scenario: Rate limited
- **WHEN** Trello returns a 429 status
- **THEN** the CLI prints "rate limited by Trello, try again later" and exits non-zero
