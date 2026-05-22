## 1. Storage — CompactionStore

- [x] 1.1 Add `CompactionRecord` struct to `src/memory.rs` with fields: `ticket_id`, `summary`, `compacted_through` (DateTime<Utc>), `compacted_count` (usize), `updated_at`
- [x] 1.2 Add `CompactionStore` struct to `src/memory.rs` that opens the same `memory.db` file and creates the `comment_compaction` table on init
- [x] 1.3 Implement `CompactionStore::set(ticket_id, summary, compacted_through, compacted_count)` using `INSERT OR REPLACE`
- [x] 1.4 Implement `CompactionStore::get(ticket_id) -> Option<CompactionRecord>`
- [x] 1.5 Implement `CompactionStore::delete(ticket_id)` — no-op if record does not exist
- [x] 1.6 Write unit tests for `CompactionStore`: set/get roundtrip, overwrite, delete, delete non-existent

## 2. Models

- [x] 2.1 Add `CommentCompaction` struct to `src/models.rs` with fields: `summary: String`, `compacted_through: DateTime<Utc>`, `compacted_count: usize`
- [x] 2.2 Add `comment_compaction: Option<CommentCompaction>` and `compaction_suggested: bool` to `Ticket` struct (both `#[serde(skip_serializing_if)]` when absent/false)

## 3. Config

- [x] 3.1 Add `comment_compaction_threshold: Option<usize>` to `AppConfig` in `src/config.rs`
- [x] 3.2 Add helper method `AppConfig::compaction_threshold() -> usize` returning the value or default 5

## 4. Commands — compact and decompact

- [x] 4.1 Add `Compact { id: String, summary: String }` variant to `TicketCommands` in `src/main.rs` with clap annotations
- [x] 4.2 Add `Decompact { id: String }` variant to `TicketCommands` in `src/main.rs` with clap annotations
- [x] 4.3 Implement `Compact` dispatch: validate summary non-empty, fetch ticket from board, call `CompactionStore::set` with boundary set to most recent comment's timestamp, print success
- [x] 4.4 Implement `Decompact` dispatch: call `CompactionStore::delete`, print success

## 5. Ticket Show — apply compaction

- [x] 5.1 In the `TicketCommands::Show` dispatch in `src/main.rs`, open `CompactionStore` after fetching the ticket
- [x] 5.2 If a compaction record exists: filter `ticket.comments` to only those with `at > compacted_through`, set `ticket.comment_compaction` from the record
- [x] 5.3 If no compaction record and `comments.len() > config.compaction_threshold()`: set `ticket.compaction_suggested = true`
- [x] 5.4 Update `print_ticket_detail` to render `comment_compaction` summary section and `compaction_suggested` hint in human-readable output

## 6. Tests

- [x] 6.1 Unit test: compaction applied correctly — comments before boundary excluded, comments after included
- [x] 6.2 Unit test: `compaction_suggested` set when over threshold and no record
- [x] 6.3 Unit test: `compaction_suggested` not set when compaction record exists
- [x] 6.4 Unit test: `compaction_suggested` not set when under threshold
- [x] 6.5 Verify all existing tests still pass after model changes
