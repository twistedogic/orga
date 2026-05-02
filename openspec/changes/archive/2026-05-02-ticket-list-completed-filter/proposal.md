## Why

`orga ticket list` currently only returns open tickets, giving agents no visibility into completed work. Agents need to query completed tickets to understand history, avoid re-creating resolved work, and report on progress.

## What Changes

- Add `completed: bool` field to the `Ticket` model
- Map `completed` from `card.closed` in the Trello backend (`TrelloCard` gains a `closed` field)
- Change `list_assigned` to fetch `filter=all` from Trello instead of `filter=open`
- Add `--completed` flag to `orga ticket list` to show only completed tickets
- Add `--all` flag to `orga ticket list` to show all tickets regardless of status
- Default behaviour (no flags) remains: show only non-completed tickets

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `board-abstraction`: `Ticket` gains a `completed: bool` field; `list_assigned` contract now returns all tickets (open and closed)
- `cli-commands`: `orga ticket list` gains `--completed` and `--all` flags

## Impact

- `src/models.rs` — add `completed: bool` to `Ticket`
- `src/board/trello.rs` — add `closed: bool` to `TrelloCard`, map to `Ticket::completed`, change API filter to `all`
- `src/main.rs` — add `--completed` / `--all` args to `TicketCommands::List`, filter results client-side
- `tests/integration_test.rs` — update `MockBoard::list_assigned` to return tickets with `completed` set, add filter tests
- No config changes, no breaking changes to `--json` output shape (field addition is additive)
