## MODIFIED Requirements

### Requirement: Ticket data model
The `Board` trait SHALL operate on a shared `Ticket` type that is backend-agnostic. The `Ticket` type SHALL include: `id`, `title`, `description`, `list_id`, `list_name`, `url`, `completed` (bool), `assignees` (Vec of Members), `sub_tickets` (Vec of TicketSummary), and `comments` (Vec of Comment). The `completed` field SHALL be `true` when the ticket is closed/archived on the backend. The `Checklist` and `ChecklistItem` types are removed.

#### Scenario: Ticket serialization
- **WHEN** a ticket is returned from any backend
- **THEN** it can be serialized to JSON using the shared type without backend-specific fields leaking

#### Scenario: Completed ticket serialization
- **WHEN** a closed/archived ticket is returned from any backend
- **THEN** its JSON representation includes `"completed": true`

#### Scenario: Open ticket serialization
- **WHEN** an open ticket is returned from any backend
- **THEN** its JSON representation includes `"completed": false`

#### Scenario: sub_tickets always present
- **WHEN** a ticket is serialized to JSON
- **THEN** `sub_tickets` is always present as an array (empty if none)

## REMOVED Requirements

### Requirement: Checklist methods on Board trait
**Reason**: Checklists are replaced by the `sub_tickets` field on `Ticket`. `add_checklist_item` and `check_item` are removed from the `Board` trait.
**Migration**: Use `create_sub` to add sub-tickets. Use `move_ticket` on a sub-ticket ID to mark it complete.
