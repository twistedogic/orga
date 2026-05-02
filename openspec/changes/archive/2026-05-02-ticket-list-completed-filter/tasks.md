## 1. Model Layer

- [x] 1.1 Add `completed: bool` field to `Ticket` struct in `src/models.rs`

## 2. Trello Backend

- [x] 2.1 Add `closed: bool` field to `TrelloCard` struct in `src/board/trello.rs`
- [x] 2.2 Map `card.closed` to `ticket.completed` in `card_to_ticket`
- [x] 2.3 Change `list_assigned` API call from `filter=open` to `filter=all`

## 3. CLI

- [x] 3.1 Add `--completed` and `--all` flags to `TicketCommands::List` in `src/main.rs`
- [x] 3.2 Apply client-side filter in the `List` handler: default open-only, `--completed` completed-only, `--all` unfiltered
- [x] 3.3 Enforce mutual exclusivity of `--completed` and `--all` with a usage error

## 4. Tests

- [x] 4.1 Update `MockBoard::list_assigned` in `tests/integration_test.rs` to return tickets with `completed` set
- [x] 4.2 Add tests for default filter (open only), `--completed`, and `--all` flag behaviour
