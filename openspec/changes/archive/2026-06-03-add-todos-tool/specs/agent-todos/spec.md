## ADDED Requirements

### Requirement: Per-scope persisted todo list
Each agent (main agent and each subagent) SHALL maintain an independent todo list stored in `MemoryStore` under a scoped key of the form `__todos_<scope>__`, where `<scope>` is `"main"` for the main agent and the sanitized subagent name for subagents. Todos SHALL persist across invocations of the same agent on the same ticket.

#### Scenario: Main agent todos isolated from subagent todos
- **WHEN** the main agent calls `todos()` and a subagent named `researcher` also calls `todos()`
- **THEN** the main agent's list is stored at `__todos_main__` and the subagent's list at `__todos_researcher__`, with no overlap

#### Scenario: Todos persist across invocations
- **WHEN** an agent sets todos in one invocation and the agent loop runs again on the same ticket
- **THEN** the previously stored todos are loaded from `MemoryStore` and used as the current list baseline

#### Scenario: Scope key is sanitized
- **WHEN** a subagent name contains characters outside `[a-zA-Z0-9_]`
- **THEN** those characters are replaced with `_` when forming the storage key

### Requirement: Replace-all semantics
The `todos` tool SHALL accept the full updated list on each call and replace the stored list entirely. Partial updates are not supported.

#### Scenario: Full list replacement
- **WHEN** the agent calls `todos([{content: "A", status: "completed"}, {content: "B", status: "in_progress"}])`
- **THEN** the stored list is replaced with exactly those two items

### Requirement: Three-status model
Each todo item SHALL have exactly one of three statuses: `pending`, `in_progress`, or `completed`. Any other status value SHALL return an error.

#### Scenario: Invalid status rejected
- **WHEN** an item with `status: "done"` is passed
- **THEN** the tool returns an error without modifying the stored list

### Requirement: Transition tracking in response
The tool response SHALL include counts of pending, in_progress, and completed items, and SHALL identify items that just transitioned to `completed` or `in_progress` by diffing against the previously stored list.

#### Scenario: Completion summary returned
- **WHEN** the agent updates the list
- **THEN** the response includes `"Status: N pending, N in progress, N completed"` and the fixed closing sentence: `"Todos have been modified successfully. Ensure that you continue to use the todo list to track your progress. Please proceed with the current tasks if applicable."`

### Requirement: Corrupt or missing storage treated as empty
If no todos exist in `MemoryStore` for the current scope (new ticket, first call), or the stored value cannot be parsed, the baseline list SHALL be treated as empty with no error.

#### Scenario: First call starts fresh
- **WHEN** a agent calls `todos()` for the first time on a ticket
- **THEN** the tool has no previous list to diff against, treats baseline as empty, and stores the new list
