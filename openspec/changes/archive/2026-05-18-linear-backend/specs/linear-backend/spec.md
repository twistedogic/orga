## ADDED Requirements

### Requirement: LinearBackend implements Board trait
The system SHALL provide a `LinearBackend` struct in `src/board/linear.rs` that fully implements the `Board` trait via Linear's GraphQL API. Every trait method SHALL be implemented — no stubs or unimplemented panics.

#### Scenario: All Board methods available
- **WHEN** `board.backend = "linear"` is configured
- **THEN** every CLI command (list, show, comment, assign, move, sub, checklist, columns, whoami, return) executes successfully against the Linear API

### Requirement: Linear config section
The system SHALL accept a `[linear]` section in the TOML config containing a single required field: `api_key`. No `member_id` is required.

#### Scenario: Valid Linear config loads
- **WHEN** the config contains `board.backend = "linear"` and a `[linear]` section with `api_key`
- **THEN** the config loads without error and `LinearBackend` is instantiated

#### Scenario: Missing linear section fails
- **WHEN** `board.backend = "linear"` but no `[linear]` section is present
- **THEN** the CLI exits with a config error referencing the missing `[linear]` section

#### Scenario: Linear backend recognized
- **WHEN** `board.backend = "linear"` is set
- **THEN** config validation passes (linear is a known backend)

### Requirement: Viewer identity auto-resolution
The system SHALL resolve the authenticated user's identity by issuing a `viewer` GraphQL query at startup. No `member_id` config field is required for Linear.

#### Scenario: Viewer resolved from API key
- **WHEN** `LinearBackend` is constructed with a valid `api_key`
- **THEN** the backend stores the viewer's `id`, `username` (display name), and resolves `whoami` correctly

#### Scenario: Invalid API key fails at startup
- **WHEN** the API key is invalid
- **THEN** the backend returns `OrgaError::Unauthorized`

### Requirement: Team as board primitive
The system SHALL use a Linear **Team** as the board primitive. `board.id` in the config SHALL be a Linear team ID. Workflow states, issue assignment, and column listing are all scoped to that team.

#### Scenario: list_columns returns team workflow states
- **WHEN** `list_columns()` is called
- **THEN** it returns the team's workflow states as `Vec<Column>` with `id` and `name`

#### Scenario: list_assigned scopes to team
- **WHEN** `list_assigned()` is called
- **THEN** it returns only issues assigned to the viewer that belong to the configured team

### Requirement: Sub-issues as checklists
The system SHALL unify `add_checklist_item` and `create_sub` under sub-issues in Linear. Both operations SHALL create a Linear sub-issue under the parent. `check_item` SHALL transition the sub-issue to the team's first workflow state with `type: completed`.

#### Scenario: add_checklist_item creates sub-issue
- **WHEN** `add_checklist_item(parent_id, title)` is called
- **THEN** a sub-issue with the given title is created under `parent_id` and its Linear issue ID is returned

#### Scenario: check_item completes sub-issue
- **WHEN** `check_item(ticket_id, item_id)` is called
- **THEN** the sub-issue with `item_id` is transitioned to the team's completed workflow state

#### Scenario: check_item fails if no completed state
- **WHEN** `check_item` is called and the team has no workflow state with `type: completed`
- **THEN** `OrgaError::BackendError` is returned with a message listing available states

#### Scenario: create_sub creates sub-issue and returns Ticket
- **WHEN** `create_sub(parent_id, title)` is called
- **THEN** a sub-issue is created and returned as a full `Ticket`

### Requirement: Sub-issues surface in get_ticket checklists
The system SHALL populate the `checklists` field of `Ticket` with sub-issues when fetching a Linear issue. Sub-issues SHALL appear as a single synthetic checklist named `"Sub-tasks"`.

#### Scenario: get_ticket with sub-issues
- **WHEN** `get_ticket(id)` is called for an issue that has sub-issues
- **THEN** `ticket.checklists` contains one entry named `"Sub-tasks"` with one `ChecklistItem` per sub-issue

#### Scenario: ChecklistItem complete reflects sub-issue state
- **WHEN** a sub-issue has a workflow state with `type: completed`
- **THEN** its `ChecklistItem.complete` is `true`

#### Scenario: get_ticket without sub-issues
- **WHEN** `get_ticket(id)` is called for an issue with no sub-issues
- **THEN** `ticket.checklists` is empty

### Requirement: Agent comment tagging preserved
The system SHALL append and parse `_[orga:name]_` tags on Linear comments identically to the Trello backend.

#### Scenario: Comment tagged with agent name
- **WHEN** `comment(id, text)` is called
- **THEN** the posted Linear comment body ends with `\n\n_[orga:<agent_name>]_`

#### Scenario: last_commenter_is_agent detection
- **WHEN** the most recent comment on an issue was posted by orga (contains the tag)
- **THEN** `TicketSummary.last_commenter_is_agent` is `true`

### Requirement: assign resolves user by display name
The system SHALL resolve a Linear user by display name when `assign` is called. The leading `@` SHALL be stripped before lookup.

#### Scenario: Valid username assigns user
- **WHEN** `assign(id, "@alice")` is called and a user with display name `alice` exists
- **THEN** the issue is assigned to that user

#### Scenario: Unknown username returns NotFound
- **WHEN** `assign(id, username)` is called and no matching user exists
- **THEN** `OrgaError::NotFound` is returned

### Requirement: return_ticket assigns back to creator
The system SHALL implement `return_ticket` by optionally posting a comment then assigning the issue back to its creator, using the `creator` field from the Linear issue.

#### Scenario: return_ticket with comment
- **WHEN** `return_ticket(id, Some("needs review"))` is called
- **THEN** a comment is posted and the issue is assigned back to the creator

#### Scenario: return_ticket without creator fails
- **WHEN** the issue has no creator
- **THEN** `OrgaError::BackendError` is returned
