## ADDED Requirements

### Requirement: Project scoping

The system SHALL accept an optional `project_id` field in the `[linear]`
config section. When set, it SHALL additionally narrow the agent's inbound
ticket queue to issues that belong to that Linear project. The project filter
SHALL be intersected with the existing team and assignee filters; the team
SHALL remain the board primitive and the sole source of workflow states and
columns. When `project_id` is unset, behavior SHALL be identical to a config
without the field.

`get_ticket`, `comment`, `assign`, and `return_ticket` SHALL NOT be gated by
`project_id`; they SHALL operate on any ticket id regardless of project
membership. Project scope is a filter on what enters the agent's queue, not a
permission boundary on direct operations.

When `create_sub` creates a sub-issue and `project_id` is set, the new
sub-issue SHALL be attached to that project (via `issueCreate`'s `projectId`
input), so that sub-tasks the agent creates remain visible to its own scoped
queue. When `project_id` is unset, no `projectId` SHALL be passed.

The `orga init board` (linear) flow SHALL offer an optional project picker
scoped to the selected team, offering "None" plus the team's projects. The
config SHALL store the project's stable UUID, never its (renameable) name.

#### Scenario: list_assigned narrowed to a configured project
- **WHEN** `project_id` is set and `list_assigned()` is called
- **THEN** the returned issues are those assigned to the viewer, in the
  configured team, AND in the configured project

#### Scenario: no project_id preserves current behavior
- **WHEN** `project_id` is unset (absent from config)
- **THEN** `list_assigned` returns the same set as before this requirement
  (team + assignee, no project clause)

#### Scenario: direct ticket lookup ignores project scope
- **WHEN** `get_ticket(id)` is called for a ticket that is not in the
  configured project
- **THEN** the ticket is still returned (project scope does not gate direct-id
  operations)

#### Scenario: comment on an out-of-project ticket succeeds
- **WHEN** `comment(id, text)` is called for a ticket not in the configured
  project
- **THEN** the comment is posted (project scope does not gate mutating
  direct-id operations)

#### Scenario: created sub-issue is attached to the project
- **WHEN** `project_id` is set and `create_sub(parent_id, …)` is called
- **THEN** the `issueCreate` input includes `projectId` equal to the configured
  `project_id`

#### Scenario: created sub-issue omits projectId when unscoped
- **WHEN** `project_id` is unset and `create_sub(parent_id, …)` is called
- **THEN** the `issueCreate` input does not include `projectId`

#### Scenario: team remains the columns source
- **WHEN** `project_id` is set and `list_columns()` is called
- **THEN** the returned columns are the team's workflow states (a project
  contributes no columns)

#### Scenario: init stores the project UUID, not the name
- **WHEN** the operator selects a project named "Q3 Migration" in
  `orga init board`
- **THEN** the written `[linear]` section contains `project_id = "<uuid>"`,
  never the literal name "Q3 Migration"

#### Scenario: init offers None and preserves existing value
- **WHEN** `orga init board` is re-run on a config that already has a
  `project_id`
- **THEN** the project picker defaults its cursor to the existing project, and
  selecting "None" removes the field from the written config
