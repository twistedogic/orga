## ADDED Requirements

### Requirement: Board trait whoami method
The `Board` trait SHALL define a `whoami(&self) -> Result<Member, OrgaError>` method. `TrelloBackend` SHALL implement it by calling `GET /1/members/{member_id}?fields=id,username,fullName` using the configured `member_id`.

#### Scenario: Resolved successfully
- **WHEN** `whoami` is called with valid credentials
- **THEN** a `Member` with `id`, `username`, and `full_name` is returned

#### Scenario: Unauthorized
- **WHEN** credentials are invalid
- **THEN** an `OrgaError::Unauthorized` is returned

### Requirement: Board trait return_ticket method
The `Board` trait SHALL define a `return_ticket(&self, id: &str, comment: Option<&str>) -> Result<(), OrgaError>` method. `TrelloBackend` SHALL implement it by: (1) fetching the ticket to read its creator, (2) if `comment` is `Some`, posting the comment (with agent tag prepended by the caller), (3) reassigning the ticket to the creator using their `username`.

#### Scenario: Return with comment
- **WHEN** `return_ticket` is called with a comment
- **THEN** comment is posted before reassignment

#### Scenario: Return without comment
- **WHEN** `return_ticket` is called with `None` for comment
- **THEN** only the reassignment is performed

#### Scenario: No creator
- **WHEN** `return_ticket` is called and the ticket has no creator
- **THEN** `OrgaError::BackendError("ticket has no known creator")` is returned

### Requirement: Creator fetched in get_ticket
`TrelloBackend::get_ticket` SHALL request `actions=commentCard,createCard` from the Trello API. It SHALL extract the `createCard` action's `memberCreator` as the ticket's creator.

#### Scenario: createCard action present
- **WHEN** `get_ticket` is called and the card has a `createCard` action
- **THEN** `Ticket.creator` is populated with the `memberCreator` from that action

#### Scenario: createCard action absent
- **WHEN** `get_ticket` is called and no `createCard` action is returned
- **THEN** `Ticket.creator` is `None`

## MODIFIED Requirements

### Requirement: Trello backend implements Board trait
The system SHALL provide a `TrelloBackend` struct implementing the `Board` trait using the Trello REST API v1. All operations SHALL use the `api_key` and `token` from config for authentication. The trait SHALL include `whoami`, `return_ticket` in addition to existing methods.

#### Scenario: Authenticated request
- **WHEN** any Trello API call is made
- **THEN** the request includes `key` and `token` query parameters from the config
