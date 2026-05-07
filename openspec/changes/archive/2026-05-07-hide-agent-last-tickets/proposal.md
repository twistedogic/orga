## Why

When an agent posts a comment on a ticket, it is waiting for a human response — the ticket is "in the agent's court." Including these tickets in the default `ticket list` output creates noise and can mislead agents into re-commenting on tickets they already responded to. Filtering them out by default makes `ticket list` a reliable "what needs my attention now" view.

## What Changes

- `ticket list` (default, no flags) SHALL hide tickets where the latest comment was posted by an agent (i.e. has an `agent_name`)
- `ticket list --all` already bypasses all filtering and continues to show all tickets regardless of comment state
- `TicketSummary` gains a `last_commenter_is_agent` boolean field to expose this signal
- `list_assigned` in the Trello backend fetches `commentCard` actions inline (alongside existing `createCard`) to populate the new field without additional API calls

## Capabilities

### New Capabilities

- `ticket-list-filtering`: The `ticket list` command filters out tickets where the most recent comment was posted by an agent, making the default output reflect only tickets that need a response.

### Modified Capabilities

- `cli-commands`: The `ticket list` default scenario changes — previously showed all non-completed assigned tickets; now hides agent-last tickets by default.

## Impact

- `src/models.rs` — add `last_commenter_is_agent: bool` to `TicketSummary`
- `src/board/trello.rs` — update `list_assigned` to fetch `commentCard` actions and set the new field in `card_to_summary`
- `src/main.rs` — apply the new filter in the default `ticket list` path (before existing `completed`/`all` checks)
- JSON output for `ticket list` gains `last_commenter_is_agent` field on each ticket object
