## Why

The agent needs to know what columns exist on the board to make routing decisions — for example, to move tickets to the right list. Currently there's no command to enumerate board columns; the agent has to hard-code list names or infer them from existing tickets.

## What Changes

- Add `orga columns` top-level command that lists all columns (lists) on the configured board
- Add `list_columns() -> Result<Vec<Column>, OrgaError>` method to the `Board` trait
- Add `Column { id, name }` model to `models.rs`
- Implement `list_columns()` on `TrelloBackend` (wraps existing private `board_lists()`)

## Capabilities

### New Capabilities

- `columns-command`: Top-level `orga columns` CLI command that outputs board columns with their IDs and names, with `--json` support

### Modified Capabilities

- `board-abstraction`: Adding `list_columns()` to the `Board` trait and `Column` to the shared data model
- `cli-commands`: Adding the `columns` command to the CLI surface

## Impact

- `src/models.rs` — new `Column` struct
- `src/board/mod.rs` — new trait method `list_columns()`
- `src/board/trello.rs` — implement `list_columns()` using existing `board_lists()` helper
- `src/main.rs` — new `Commands::Columns` variant and handler
