## Context

`orga` exposes a `Board` trait with `add_checklist_item` and `check_item` methods, and a `Ticket` model with `checklists: Vec<Checklist>`. On Linear, sub-issues are first-class objects with id, url, and state — but the current code maps them into a `Checklist` named "Sub-tasks", losing structured fields. On Trello, there is no native sub-ticket concept; sub-tickets are created as sibling cards with a text checklist item on the parent as a linkback.

The `create-sub` CLI command accepts only a title. Agents decomposing work into sub-tickets cannot provide description or control initial placement.

## Goals / Non-Goals

**Goals:**
- Replace `checklists` with `sub_tickets: Vec<TicketSummary>` in the `Ticket` model
- Surface sub-tickets in `ticket show --json` as structured objects (id, title, url, list, completed)
- Extend `create-sub` with `--description` and `--list` (defaults to parent's list)
- Remove the `orga checklist` command group entirely
- Update the orga agent skill with decomposition workflow guidance

**Non-Goals:**
- Hydrating Trello sub-tickets from checklist linkback text (too costly, agents use `ticket show <sub_id>`)
- Assigning sub-tickets at creation time
- Nesting sub-tickets beyond one level

## Decisions

### Remove checklists entirely, not repurpose them

**Decision**: Delete `Checklist`, `ChecklistItem` from models and remove `add_checklist_item`/`check_item` from the `Board` trait.

**Rationale**: Linear already uses sub-issues for what was exposed as checklists (`add_checklist_item` calls `create_sub_issue` internally). Keeping both concepts would require mapping one onto the other in perpetuity. Sub-tickets are a richer, more structured primitive — they have ids, URLs, and states that agents can act on.

**Alternative considered**: Keep checklists as a separate concept alongside sub-tickets. Rejected — it adds surface area without value; anything worth tracking is worth being a ticket.

### Trello: `sub_tickets: vec![]` always (no hydration)

**Decision**: Trello returns an empty `sub_tickets` array in `ticket show`. Sub-tickets are explored individually via `ticket show <sub_id>`.

**Rationale**: Trello has no native parent-child relation. The current workaround stores the sub-ticket URL in a checklist text item. Parsing and re-fetching those cards would require N extra API calls per `ticket show` and is fragile. Agents can call `ticket show` on sub-ticket IDs they know; the parent's `sub_tickets` list is advisory, not exhaustive on Trello.

**Alternative considered**: Parse "Sub-tasks" checklist text and resolve card IDs from URLs. Rejected — brittle URL parsing, N+1 API calls, no benefit over just using `ticket show`.

### `--list` defaults to parent's list

**Decision**: When `--list` is omitted, the new sub-ticket is placed in the same list/state as its parent.

**Rationale**: Consistent with current Trello behavior. On Linear, this means resolving the parent's current state name and looking up its `stateId`. Simple and predictable.

### `create_sub` trait signature

```rust
fn create_sub(
    &self,
    parent_id: &str,
    title: &str,
    description: Option<&str>,
    list: Option<&str>,
) -> Result<Ticket, OrgaError>;
```

`list` is a human-readable name (e.g., "In Progress"); backends resolve to id internally using existing `list_columns()` / `team_states()` helpers.

### Post-creation agent workflow belongs in the skill, not the CLI

**Decision**: The CLI does not auto-comment on the parent after `create-sub`. The agent skill instructs agents to do this themselves.

**Rationale**: The CLI is a thin tool. Orchestration logic (comment on parent, stop and wait) belongs in the skill layer. This keeps the CLI composable.

## Risks / Trade-offs

- **Breaking change to `Ticket` JSON shape** → Any agent or script parsing `checklists` will break. Agents using this skill will receive the updated skill guidance. External integrations must update.
- **Linear list resolution requires an extra API call** → `create-sub --list` needs to call `team_states()` to map name → id. This is already used elsewhere; acceptable overhead.
- **Trello `sub_tickets` always empty** → Agents on Trello cannot discover sub-tickets from a parent. Mitigated by: agent creates sub-tickets itself and tracks their IDs in ticket memory.

## Migration Plan

1. Update `Ticket` model — remove `checklists`, add `sub_tickets`
2. Update `Board` trait — remove checklist methods, update `create_sub` signature
3. Update Linear backend — map children to `Vec<TicketSummary>`, implement new `create_sub`
4. Update Trello backend — implement new `create_sub`, return `sub_tickets: vec![]`
5. Update `src/main.rs` — remove `checklist` commands, add `--description`/`--list` to `create-sub`
6. Update agent skill — decomposition section, remove checklist section, update command reference
7. Run tests — fix any broken tests referencing checklists

No database migration needed (checklists were never persisted).
