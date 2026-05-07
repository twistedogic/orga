## Context

`ticket list` calls `list_assigned` which hits `/members/{id}/cards` with `actions=createCard`. The returned `TrelloCard` structs are mapped to `TicketSummary` via `card_to_summary` — a lightweight mapping that does not fetch comments. Full comment data is only available through `get_ticket` which hits `/cards/{id}` with `actions=commentCard,createCard`.

`TicketSummary` is the model returned by `list_assigned` and used for list rendering. It currently has no comment-related fields. The `Ticket` model (used by `ticket show`) carries the full `Vec<Comment>`, each with an `agent_name: Option<String>` that is `Some` when the comment was tagged by the orga agent.

## Goals / Non-Goals

**Goals:**
- Default `ticket list` hides tickets where the latest comment was posted by an agent
- No additional API calls per ticket (O(1) total, not O(n))
- `--all` continues to bypass all filters and show everything
- `TicketSummary` exposes `last_commenter_is_agent` for JSON consumers

**Non-Goals:**
- Filtering by *which* agent posted (any agent tag counts)
- Changing `ticket show` behavior
- Filtering based on comment content or age

## Decisions

### 1. Add `commentCard` to the `list_assigned` API query

**Decision**: Change `actions=createCard` to `actions=commentCard,createCard` in the `/members/{id}/cards` request.

Trello returns actions inline per card. Adding `commentCard` gives us all comment actions without extra calls. The `TrelloCard` struct already has an `actions` field; it will now contain both action types.

**Alternative considered**: Call `get_ticket` for each card to get comments. Rejected — N extra API calls, one per assigned ticket.

### 2. Add `last_commenter_is_agent: bool` to `TicketSummary`

**Decision**: Add a single boolean field rather than embedding comment objects or a commenter identity.

The list layer only needs to know "should this ticket be shown by default?" A boolean is the minimal sufficient signal. It also surfaces cleanly in JSON output for agent consumers who may want to use it for their own logic.

**Alternative considered**: Add `last_comment_agent_name: Option<String>`. More expressive but unnecessary for the current use case. Can be added later if needed.

### 3. Populate the field in `card_to_summary`

**Decision**: `card_to_summary` inspects the card's inline comment actions, finds the latest by date, checks for an agent tag, and sets `last_commenter_is_agent` accordingly.

`card_to_summary` already takes the `TrelloCard` (which has actions). No signature changes needed to `list_assigned`. Parsing reuses the existing `parse_agent_tag` function.

### 4. Filter in `main.rs`, not in `list_assigned`

**Decision**: The default filter (hide agent-last tickets) is applied in `main.rs` alongside the existing `completed` / `all` filters, not inside `list_assigned`.

`list_assigned` returns the full picture; filtering is a presentation/policy concern. This keeps the backend method general and the filtering logic co-located and easy to read.

## Risks / Trade-offs

- **Inline actions may be paginated or capped by Trello** → Trello returns up to 50 actions per card inline when using the `actions` param on the member cards endpoint. For tickets with very high comment volume the latest comment might not appear. In practice agent-managed tickets are unlikely to exceed this. Can be revisited if it becomes an issue.
- **`last_commenter_is_agent` becomes stale in cached contexts** → The field reflects the state at list time. Consumers should not cache it across sessions. This is consistent with orga's stateless design.

## Migration Plan

No migration needed. `last_commenter_is_agent` defaults to `false` for tickets with no comments, which is the same as the previous behavior (they would have been shown). JSON output gains a new field — additive, not breaking.
