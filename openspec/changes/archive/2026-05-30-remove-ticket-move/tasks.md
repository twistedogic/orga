## 1. Remove from Board Trait and Implementations

- [x] 1.1 Remove `move_ticket` method from `Board` trait in `src/board/mod.rs`
- [x] 1.2 Remove `move_ticket` implementation from `src/board/trello.rs`
- [x] 1.3 Remove `move_ticket` implementation from `src/board/linear.rs`
- [x] 1.4 Remove mock `move_ticket` impl from `tests/integration_test.rs`

## 2. Remove CLI Subcommand

- [x] 2.1 Remove `Move` variant from `TicketCommands` enum in `src/main.rs`
- [x] 2.2 Remove the `TicketCommands::Move` match arm in `src/main.rs`

## 3. Remove Agent Tool

- [x] 3.1 Remove `"move_ticket"` dispatch branch from `dispatch()` in `src/agent/tools.rs`
- [x] 3.2 Remove `MoveTicketArgs` struct from `src/agent/tools.rs`
- [x] 3.3 Remove `dispatch_move_ticket()` function from `src/agent/tools.rs`
- [x] 3.4 Remove `move_ticket` entry from the tool schema array in `src/agent/tools.rs`
- [x] 3.5 Remove `dispatch_move_ticket_dry_run` test from `src/agent/tools.rs`

## 4. Update Agent Skill

- [x] 4.1 Audit `skills/orga/SKILL.md` for any mention of ticket movement and remove/update

## 5. Verify

- [x] 5.1 Run `cargo build` — must compile with zero errors
- [x] 5.2 Run `cargo test` — all tests must pass
- [x] 5.3 Run `cargo clippy` — no new warnings
