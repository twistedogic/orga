## MODIFIED Requirements

### Requirement: Ticket data model
The `Board` trait SHALL operate on a shared `Ticket` type that is backend-agnostic. The `Ticket` type SHALL include: `id`, `title`, `description`, `list_id`, `list_name`, `url`, `completed` (bool), `assignees` (Vec of usernames), `checklists` (Vec of checklist with items), and `comments` (Vec of Comment). The `completed` field SHALL be `true` when the ticket is closed/archived on the backend.

#### Scenario: Ticket serialization
- **WHEN** a ticket is returned from any backend
- **THEN** it can be serialized to JSON using the shared type without backend-specific fields leaking

#### Scenario: Completed ticket serialization
- **WHEN** a closed/archived ticket is returned from any backend
- **THEN** its JSON representation includes `"completed": true`

#### Scenario: Open ticket serialization
- **WHEN** an open ticket is returned from any backend
- **THEN** its JSON representation includes `"completed": false`

## ADDED Requirements

### Requirement: list_assigned returns all tickets
The `list_assigned` method on the `Board` trait SHALL return all tickets assigned to the agent, regardless of completion state. Filtering by completion state is the caller's responsibility.

#### Scenario: Mix of open and completed tickets
- **WHEN** the agent has both open and closed tickets assigned
- **THEN** `list_assigned` returns all of them with `completed` set correctly on each
