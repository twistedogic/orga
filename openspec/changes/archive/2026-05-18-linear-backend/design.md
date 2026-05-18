## Context

orga has a clean `Board` trait abstraction with a single backend today (Trello, REST). Adding Linear requires implementing that same trait against Linear's GraphQL API. The abstraction is already in place — this is purely an additive backend.

Key differences from Trello:
- Linear uses GraphQL (POST to `/graphql`) not REST
- Linear's equivalent of a board is a **Team** (workflow states live on teams)
- Linear has no native checklist concept — sub-issues serve both purposes
- Linear resolves the current user via `viewer` query from the API key — no `member_id` config needed
- Linear issue IDs are UUIDs (e.g. `9f4b8c3a-...`); Trello IDs are shorter alphanumeric strings

## Goals / Non-Goals

**Goals:**
- Full `Board` trait implementation for Linear — every method, no stubs
- Config parity: `board.backend = "linear"` works with a `[linear]` section containing only `api_key`
- Sub-issues used for both `create_sub` and checklist operations (`add_checklist_item` / `check_item`)
- Sub-issues surface in `get_ticket` as a synthetic checklist in the `checklists` field
- Agent comment tagging (`_[orga:name]_`) preserved unchanged

**Non-Goals:**
- Linear Projects as boards (Teams only)
- Linear attachments, labels, priorities, or any field beyond the shared `Ticket`/`TicketSummary` model
- Async HTTP — blocking `reqwest` throughout, consistent with existing code

## Decisions

### GraphQL transport layer

**Decision**: Implement a minimal `gql<T>(query, variables)` method on `LinearBackend` that POSTs to `https://api.linear.app/graphql` with `Authorization: Bearer <api_key>` and deserializes the `data` field.

**Rationale**: Linear's API is GraphQL-only. Rather than pulling in a GraphQL client crate, a thin wrapper over `reqwest` keeps the dependency footprint zero and matches the pattern in `TrelloBackend` (which also has private HTTP helpers). Error handling maps `errors[0].message` to `OrgaError::BackendError`.

**Alternative considered**: `cynic` or `graphql_client` crates — rejected because they add build complexity (codegen) and the query surface we need is small.

---

### Board primitive: Team (not Project)

**Decision**: `board.id` is a Linear team ID. All operations scope to that team.

**Rationale**: Workflow states (= columns) are owned by teams in Linear. `list_columns` returns `team.states`. `list_assigned` filters to issues in the team. Projects are ephemeral and don't own workflow states — they can't map cleanly to the `Board` abstraction.

---

### Checklist ↔ Sub-issue unification

**Decision**: `add_checklist_item(parent_id, title)` creates a sub-issue and returns its ID. `check_item(ticket_id, item_id)` updates that sub-issue's state to the team's first "completed" (`type: completed`) workflow state. `create_sub` does the same but returns a full `Ticket`. `get_ticket` collects sub-issues and populates `checklists` as a single synthetic checklist named `"Sub-tasks"` with one `ChecklistItem` per sub-issue (`complete = sub.state.type == "completed"`).

**Rationale**: Linear has no checklist primitive. Sub-issues are full issues with state, so "completing" a checklist item maps to transitioning the sub-issue to a completed state. This keeps the `Ticket` model consistent across backends — callers see `checklists` populated either way.

**Alternative considered**: Embedding markdown checkboxes in the issue description — rejected because parsing/editing description text is fragile and loses structured IDs.

---

### Viewer identity auto-resolution

**Decision**: On `LinearBackend::new`, issue a `viewer { id username displayName }` query and store the result. No `member_id` in `[linear]` config.

**Rationale**: Linear API keys are user-scoped (or OAuth). The `viewer` query always returns the authenticated identity. This eliminates a config field that is easy to get wrong.

---

### `return_ticket` and creator detection

**Decision**: Use `issue.creator { id username displayName }` from the GraphQL response. `return_ticket` assigns the issue back to the creator and optionally posts a comment — same logic as Trello.

**Rationale**: Linear issues carry a `creator` field natively, matching the Trello pattern.

---

### `assign` method

**Decision**: `assign(id, username)` resolves the Linear user by querying `users(filter: { displayName: { eq: username } })` (stripping leading `@`), then calls `issueUpdate` mutation with `assigneeId`.

**Rationale**: Linear doesn't have a direct "lookup by username" endpoint like Trello's `/members/{username}`. Display name is the closest match. If zero or multiple results are found, return `OrgaError::NotFound` or `OrgaError::BackendError` respectively.

## Risks / Trade-offs

- **Display name ambiguity in `assign`**: Two users with the same display name → `BackendError`. Mitigation: document this limitation; a future improvement could accept Linear user IDs directly.
- **`check_item` requires a completed state**: If a team has no workflow state with `type: completed`, `check_item` will error. Mitigation: surface a clear error message listing available states.
- **Sub-issue ordering in checklist**: Linear sub-issues are returned in creation order by default. No sort guarantee if issues are reordered manually. Mitigation: sort by `createdAt` in the response.
- **GraphQL query depth**: Fetching sub-issues inside `get_ticket` adds a nested query. Linear's API handles this fine for typical issue sizes. No pagination needed for the expected sub-issue count.

## Migration Plan

Config change required for users switching to Linear:

```toml
[board]
backend = "linear"
id = "<team-id>"

[linear]
api_key = "lin_api_..."
```

No data migration — memory store keys are ticket IDs which remain stable. Trello config is simply unused when backend is `"linear"`.
