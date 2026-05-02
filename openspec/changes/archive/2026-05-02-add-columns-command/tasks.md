## 1. Data Model

- [x] 1.1 Add `Column { id: String, name: String }` struct to `src/models.rs` with `Debug`, `Clone`, `Serialize`, `Deserialize` derives

## 2. Board Trait

- [x] 2.1 Add `list_columns() -> Result<Vec<Column>, OrgaError>` method to the `Board` trait in `src/board/mod.rs`

## 3. Trello Backend

- [x] 3.1 Implement `list_columns()` on `TrelloBackend` in `src/board/trello.rs`, mapping existing `board_lists()` output to `Vec<Column>`

## 4. CLI Command

- [x] 4.1 Add `Commands::Columns` variant to the `Commands` enum in `src/main.rs` with `about = "List all columns on the board"`
- [x] 4.2 Add handler in `run()`: call `board.list_columns()`, print `<id>\t<name>` per line (human) or JSON array (--json)

## 5. Verification

- [x] 5.1 Run `cargo build` and confirm it compiles without errors or warnings
- [x] 5.2 Run `cargo test` and confirm all tests pass
