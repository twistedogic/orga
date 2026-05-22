## Context

`ticket show` currently returns all comments on a ticket in a flat `Vec<Comment>`. There is no upper bound. For long-running tickets with many comments, agents receive unbounded context that grows linearly with board activity. The `MemoryStore` already provides a SQLite-backed per-ticket key-value store; this change extends that storage pattern with a dedicated compaction table.

## Goals / Non-Goals

**Goals:**
- Store an agent-written summary and a compaction boundary (timestamp) per ticket
- Return only comments after the boundary on `ticket show`, alongside the stored summary
- Signal to agents when a ticket is a compaction candidate (uncompacted count exceeds threshold)
- Allow manual reset via `decompact`
- Keep orga LLM-agnostic — the agent supplies the summary text

**Non-Goals:**
- orga does not generate summaries itself; no LLM integration
- Compaction is not automatic or triggered by a daemon
- Compaction records are not shared on the board — they remain local to the agent's SQLite store
- No compaction of the ticket description or checklists

## Decisions

### 1. Extend `memory.rs` with a new `CompactionStore` backed by the same SQLite file

**Decision:** Add a `CompactionStore` struct in `memory.rs` that opens the same `memory.db` file and manages a `comment_compaction` table. Do not merge it into `MemoryStore`.

**Alternatives considered:**
- Merge into `MemoryStore`: would conflate two distinct concerns and complicate the existing simple API
- Separate SQLite file: unnecessary fragmentation for small data

### 2. Compaction boundary is `compacted_through: DateTime<Utc>`

**Decision:** Store the ISO8601 timestamp of the most recent comment at the time `compact` is called. On `ticket show`, any comment with `at <= compacted_through` is excluded from the `comments` array.

**Alternatives considered:**
- Store a comment ID: brittle across backends (IDs differ between Trello and Linear)
- Store a count offset: breaks if comments are deleted upstream

### 3. Apply compaction in `main.rs` dispatch, not in the `Board` trait or models

**Decision:** After `board.get_ticket()` returns, `main.rs` loads the compaction record from `CompactionStore` and transforms the `Ticket` before rendering or serializing it. The `Ticket` struct gains two new optional fields: `comment_compaction: Option<CommentCompaction>` and `compaction_suggested: bool`.

**Alternatives considered:**
- Apply in `Board::get_ticket`: would require every backend to know about local SQLite state — wrong layer
- Apply in a model method: cleaner but makes `Ticket` depend on storage, breaking the model's purity

### 4. `compaction_suggested` is computed fresh on every `ticket show`

**Decision:** After applying any stored compaction, count remaining comments. If the count exceeds `comment_compaction_threshold` (config, default 5) and no compaction record exists, set `compaction_suggested: true`.

**Note:** If a compaction record exists, `compaction_suggested` is always `false` — the agent already acted.

### 5. `decompact` deletes the row; `compact` upserts

**Decision:** `compact` uses `INSERT OR REPLACE` semantics. `decompact` does a hard `DELETE`. No soft-delete or history.

## Risks / Trade-offs

- **Stale compaction boundary** → If comments are retroactively deleted upstream, the stored `compacted_through` may reference a comment that no longer exists. The timestamp boundary still filters correctly since it's date-based, not ID-based. Low risk.
- **Compaction applied per-agent** → Two agents running against the same ticket each have independent compaction records. They may have different views of comment history. Acceptable — memory is explicitly local and private.
- **No migration needed** → `CREATE TABLE IF NOT EXISTS` handles first-run. Existing `memory.db` files gain the new table transparently.
