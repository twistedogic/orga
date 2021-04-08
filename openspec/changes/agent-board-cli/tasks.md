## 1. Project Setup

- [x] 1.1 Add dependencies to `Cargo.toml`: `clap` (derive), `serde`, `serde_json`, `toml`, `reqwest` (blocking), `rusqlite`, `chrono`, `thiserror`
- [x] 1.2 Define shared data model types: `Ticket`, `Comment`, `Checklist`, `ChecklistItem`, `Member` in `src/models.rs`
- [x] 1.3 Define `OrgaError` enum in `src/error.rs` covering: `NotFound`, `Unauthorized`, `RateLimited`, `NetworkError`, `BackendError`, `ConfigError`

## 2. Config

- [x] 2.1 Define config structs (`AgentConfig`, `BoardConfig`, `TrelloConfig`, `MemoryConfig`, `AppConfig`) in `src/config.rs`
- [x] 2.2 Implement config loading from file path with serde/toml deserialization
- [x] 2.3 Implement config path resolution: `--config` flag → `ORGA_CONFIG` env var → `~/.orga/config.toml`
- [x] 2.4 Implement config validation: required fields, backend name whitelist

## 3. Board Abstraction

- [x] 3.1 Define `Board` trait in `src/board/mod.rs` with all required methods returning `Result<T, OrgaError>`
- [x] 3.2 Implement backend factory function that resolves the correct backend from config

## 4. Trello Backend

- [x] 4.1 Create `src/board/trello.rs` with `TrelloBackend` struct holding API key, token, board ID, member ID
- [x] 4.2 Implement `list_assigned`: fetch cards for member filtered by board
- [x] 4.3 Implement `get_ticket`: fetch card by ID with all details (checklists, comments/actions)
- [x] 4.4 Implement `comment`: post a comment on a card
- [x] 4.5 Implement `assign`: resolve username to member ID, add member to card
- [x] 4.6 Implement `move_ticket`: resolve list name to list ID, update card's `idList`
- [x] 4.7 Implement `create_sub`: create child card in same list, add "Sub-tasks" checklist item on parent with link
- [x] 4.8 Implement `add_checklist_item`: create or reuse a checklist on the card, add item
- [x] 4.9 Implement `check_item`: mark checklist item complete via Trello API
- [x] 4.10 Add HTTP 429 detection and return `OrgaError::RateLimited`

## 5. Memory Store

- [x] 5.1 Create `src/memory.rs` with `MemoryStore` struct backed by SQLite
- [x] 5.2 Implement auto-initialization: create `~/.orga/` dir and `memory.db` on first use
- [x] 5.3 Implement `set(ticket_id, context)` — upsert into `memory` table
- [x] 5.4 Implement `get(ticket_id)` — retrieve context by ticket ID, return `None` if absent

## 6. CLI Commands

- [x] 6.1 Set up `clap` command structure in `src/main.rs` with global `--config` and `--json` flags
- [x] 6.2 Implement `orga ticket list` command
- [x] 6.3 Implement `orga ticket show <id>` command
- [x] 6.4 Implement `orga ticket comment <id> <text>` command
- [x] 6.5 Implement `orga ticket assign <id> <username>` command
- [x] 6.6 Implement `orga ticket move <id> <list>` command
- [x] 6.7 Implement `orga ticket create-sub <parent-id> <title>` command
- [x] 6.8 Implement `orga checklist add <ticket-id> <item-text>` command
- [x] 6.9 Implement `orga checklist check <ticket-id> <item-id>` command
- [x] 6.10 Implement `orga memory set <ticket-id> <context>` command
- [x] 6.11 Implement `orga memory get <ticket-id>` command

## 7. Output Formatting

- [x] 7.1 Implement human-readable formatters for ticket list and ticket show
- [x] 7.2 Implement `--json` output path for all read commands using `serde_json`
- [x] 7.3 Implement `--json` error output: `{"error": "<message>"}` to stderr on failure

## 8. Testing

- [x] 8.1 Unit tests for config loading and validation
- [x] 8.2 Unit tests for memory store (set, get, overwrite, missing key)
- [x] 8.3 Integration tests for CLI commands using a mock `Board` trait implementation
- [x] 8.4 Verify `--json` output is valid JSON for all read commands
