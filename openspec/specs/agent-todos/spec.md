# agent-todos Specification

## Purpose
Per-scope, persisted task list tool for agents. Each agent (main agent and each subagent) maintains an independent todo list backed by `TodoStore` (SQLite), enabling structured progress tracking across multi-step tickets that survives agent restarts.

## Requirements

### Requirement: Per-scope persisted todo list
Each agent (main agent and each subagent) SHALL maintain an independent todo list stored in `TodoStore` keyed by `(ticket_id, scope)`, where `scope` is `"main"` for the main agent and the sanitized subagent name for subagents. Todos SHALL persist across invocations of the same agent on the same ticket.

#### Scenario: Main agent todos isolated from subagent todos
- **WHEN** the main agent calls `todos()` and a subagent named `researcher` also calls `todos()`
- **THEN** the main agent's list is stored under scope `"main"` and the subagent's list under scope `"researcher"`, with no overlap

#### Scenario: Todos persist across invocations
- **WHEN** an agent sets todos in one invocation and the agent loop runs again on the same ticket
- **THEN** the previously stored todos are loaded from `TodoStore` and used as the current list baseline

#### Scenario: Scope key is sanitized
- **WHEN** a subagent name contains characters outside `[a-zA-Z0-9_]`
- **THEN** those characters are replaced with `_` when forming the storage scope key

### Requirement: Replace-all semantics
The `todos` tool SHALL accept the full updated list on each call and replace the stored list entirely. Partial updates are not supported.

#### Scenario: Full list replacement
- **WHEN** the agent calls `todos([{content: "A", status: "completed"}, {content: "B", status: "in_progress"}])`
- **THEN** the stored list is replaced with exactly those two items

### Requirement: Three-status model
Each todo item SHALL have exactly one of three statuses: `pending`, `in_progress`, or `completed`. Any other status value SHALL return an error without modifying the stored list.

#### Scenario: Invalid status rejected
- **WHEN** an item with `status: "done"` is passed
- **THEN** the tool returns an error without modifying the stored list

### Requirement: Transition tracking in response
The tool response SHALL include counts of pending, in_progress, and completed items after the update.

#### Scenario: Completion summary returned
- **WHEN** the agent updates the list
- **THEN** the response includes `"Status: N pending, N in progress, N completed"` and the fixed closing sentence: `"Todos have been modified successfully. Ensure that you continue to use the todo list to track your progress. Please proceed with the current tasks if applicable."`

### Requirement: Corrupt or missing storage treated as empty
If no todos exist in `TodoStore` for the current scope (new ticket or first call), or the stored value cannot be parsed, the baseline list SHALL be treated as empty with no error.

#### Scenario: First call starts fresh
- **WHEN** an agent calls `todos()` for the first time on a ticket
- **THEN** the tool treats baseline as empty, stores the new list, and returns a success response
