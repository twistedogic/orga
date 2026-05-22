## Why

When tickets accumulate many comments, agents receive an unbounded wall of text on `ticket show`, consuming large amounts of context window. Comment compaction lets agents summarize older discussion and store it, so future reads only surface the summary plus recent comments.

## What Changes

- New `ticket compact <id> --summary "..."` command — stores an agent-written summary and marks a compaction boundary
- New `ticket decompact <id>` command — deletes the stored compaction record (manual reset)
- `ticket show` output changes to include `comment_compaction` field and only returns comments after the compaction boundary; adds `compaction_suggested: true` hint when uncompacted comment count exceeds the threshold
- New `comment_compaction_threshold` config option with default of 5

## Capabilities

### New Capabilities

- `comment-compaction`: Per-ticket compaction records stored in SQLite; commands to write and delete them; `ticket show` applies compaction to the comments list and surfaces the hint

### Modified Capabilities

- `cli-commands`: Two new subcommands (`compact`, `decompact`) added to the ticket command group

## Impact

- `src/memory.rs` — new `CompactionStore` (or extended `MemoryStore`) with new SQLite table
- `src/models.rs` — `Ticket` gains optional `comment_compaction` and `compaction_suggested` fields
- `src/main.rs` — new `Compact` and `Decompact` variants in `TicketCommands`; `ticket show` applies compaction logic
- `src/config.rs` — new `comment_compaction_threshold: Option<usize>` field
- `src/board/mod.rs` / backends — no changes required; compaction is applied after fetch
