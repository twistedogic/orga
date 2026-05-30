## Why

Ticket movement (changing a ticket's column/status) is a human decision reflecting workflow progress — it should not be automatable by agents. Removing this capability enforces a clear boundary: agents comment and observe, humans decide when work advances.

## What Changes

- **BREAKING** Remove `ticket move` CLI subcommand
- **BREAKING** Remove `move_ticket` from the `Board` trait and all implementations (Trello, Linear, mock)
- **BREAKING** Remove `move_ticket` agent tool from the dispatch table, `MoveTicketArgs` struct, and tool schema entry
- Remove associated tests for `move_ticket` agent dispatch

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `cli-commands`: `ticket move` subcommand is removed
- `agent-tools`: `move_ticket` tool is removed from the agent tool registry
- `board-abstraction`: `move_ticket` method is removed from the `Board` trait

## Impact

- `src/main.rs` — remove `Move` variant from `TicketCommands` and its match arm
- `src/board/mod.rs` — remove `move_ticket` from `Board` trait
- `src/board/trello.rs` — remove `move_ticket` implementation
- `src/board/linear.rs` — remove `move_ticket` implementation
- `src/agent/tools.rs` — remove dispatch branch, `MoveTicketArgs`, `dispatch_move_ticket`, schema entry, and dry-run test
- `tests/integration_test.rs` — remove mock `move_ticket` impl
