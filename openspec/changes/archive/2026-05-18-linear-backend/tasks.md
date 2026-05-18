## 1. Config

- [x] 1.1 Add `LinearConfig` struct to `src/config.rs` with `api_key: String`
- [x] 1.2 Add `linear: Option<LinearConfig>` field to `AppConfig`
- [x] 1.3 Update `validate()` in `config.rs` to recognize `"linear"` as a supported backend
- [x] 1.4 Add validation: if `backend = "linear"` and `[linear]` section is missing, return a config error
- [x] 1.5 Add config tests: valid linear config loads, missing `[linear]` section fails, `"linear"` passes backend validation

## 2. LinearBackend scaffold

- [x] 2.1 Create `src/board/linear.rs` with `LinearBackend` struct holding `api_key`, `team_id`, `agent_name`, `viewer` (resolved Member), `client`, `logger`
- [x] 2.2 Add `pub mod linear;` to `src/board/mod.rs`
- [x] 2.3 Implement private `gql<T>(&self, query: &str, variables: serde_json::Value) -> Result<T, OrgaError>` helper that POSTs to `https://api.linear.app/graphql` with `Authorization: Bearer <api_key>`, deserializes `data`, and maps `errors[0].message` to `OrgaError::BackendError`
- [x] 2.4 Implement `LinearBackend::new(api_key, team_id, agent_name, logger)` that issues the `viewer` query and stores the result; return `OrgaError::Unauthorized` on invalid key

## 3. Register in factory

- [x] 3.1 Add `"linear"` arm to `build_board()` in `src/board/mod.rs` that reads `LinearConfig` and constructs `LinearBackend`

## 4. Read operations

- [x] 4.1 Implement `whoami()` — return stored viewer as `Member`
- [x] 4.2 Implement `list_columns()` — query team workflow states, return `Vec<Column>`
- [x] 4.3 Implement `list_assigned()` — query issues assigned to viewer in the team, including `creator` and latest comment; map to `Vec<TicketSummary>` with `last_commenter_is_agent` detection
- [x] 4.4 Implement `get_ticket(id)` — fetch issue with comments, assignees, creator, and sub-issues; map sub-issues to single `"Sub-tasks"` checklist; sort comments by `createdAt`

## 5. Write operations

- [x] 5.1 Implement `comment(id, text)` — post comment with `_[orga:name]_` tag via `commentCreate` mutation
- [x] 5.2 Implement `move_ticket(id, list)` — resolve workflow state by name (case-insensitive), call `issueUpdate` with `stateId`
- [x] 5.3 Implement `assign(id, username)` — strip leading `@`, query users by display name, call `issueUpdate` with `assigneeId`; return `NotFound` if no match
- [x] 5.4 Implement `create_sub(parent_id, title)` — call `issueCreate` with `parentId` and `teamId`, return full `Ticket` via `get_ticket`
- [x] 5.5 Implement `add_checklist_item(parent_id, title)` — same mutation as `create_sub`, return sub-issue ID string
- [x] 5.6 Implement `check_item(ticket_id, item_id)` — find team's first workflow state with `type: completed`, call `issueUpdate` on `item_id`; error if no completed state exists
- [x] 5.7 Implement `return_ticket(id, comment)` — optionally post comment, then assign to creator; error if no creator

## 6. Tests

- [x] 6.1 Add unit tests for `append_agent_tag` and `parse_agent_tag` (copy pattern from `trello.rs`)
- [x] 6.2 Add unit test: `last_commenter_is_agent` is `true` when latest comment has orga tag
- [x] 6.3 Add unit test: `last_commenter_is_agent` is `false` when latest comment has no tag
- [x] 6.4 Add unit test: `get_ticket` maps sub-issues to `checklists[0]` with correct `complete` values
- [x] 6.5 Run `cargo test` — all tests pass
- [x] 6.6 Run `cargo clippy` — no warnings
