## Context

`orga` is a Rust CLI binary designed to be invoked by LLM agent skills. The agent is a first-class member of a shared kanban board — it gets assigned tickets, reads context, comments, creates sub-tasks, and moves tickets forward. The CLI is stateless between calls; the only persistence is a local per-ticket memory store.

The project starts fresh in Rust. The existing `src/main.rs` has a skeletal data model (`Ticket`, `Comment`, `User`, `Role`) that aligns with the direction but needs significant expansion.

## Goals / Non-Goals

**Goals:**
- A single `orga` binary with a clear, scriptable command surface
- `--json` flag on all read commands for machine-readable output
- `Board` trait that backend adapters implement — Trello is the first
- Per-ticket memory (local) that survives across agent skill invocations
- TOML config file: one agent identity, one board, one backend
- Agent permissions: comment, assign, move, create-sub, checklist — never close

**Non-Goals:**
- Running as a daemon or polling loop — the invoking skill handles scheduling
- Multiple agents per config
- Shared/team memory — memory is private to the agent running the CLI
- Building a TUI — the board UI lives in Trello (or future backends)
- Authentication flows — credentials are pre-configured in the config file

## Decisions

### 1. CLI framework: `clap` with derive macros

`clap` is the standard Rust CLI library. Derive macros keep the command definitions close to the structs and reduce boilerplate. Alternative considered: `argh` — lighter, but less ecosystem support and fewer features (`--json` flag handling, subcommands).

### 2. `Board` trait as the core abstraction

```rust
trait Board {
    fn list_assigned(&self) -> Result<Vec<Ticket>>;
    fn get_ticket(&self, id: &str) -> Result<Ticket>;
    fn comment(&self, id: &str, text: &str) -> Result<()>;
    fn assign(&self, id: &str, username: &str) -> Result<()>;
    fn move_ticket(&self, id: &str, list: &str) -> Result<()>;
    fn create_sub(&self, parent_id: &str, title: &str) -> Result<Ticket>;
    fn add_checklist_item(&self, id: &str, text: &str) -> Result<()>;
    fn check_item(&self, id: &str, item_id: &str) -> Result<()>;
}
```

Each backend is a struct implementing this trait. The CLI resolves the backend from config and dispatches through the trait. Alternative considered: enum dispatch — simpler but doesn't scale to multiple backends without touching core CLI code.

### 3. Trello as first backend via REST API

Trello has a well-documented REST API. The Trello backend will use `reqwest` (blocking) for HTTP calls and `serde_json` for deserialization. Webhook support is deferred — the skill polls via `orga ticket list`.

Assignment in Trello maps to card members. The Trello backend reads `member_id` from its own `[trello]` config section to filter `GET /members/{id}/cards`. Agent identity in `[agent]` is backend-agnostic (name only); each backend config block carries the backend-specific identity for that backend.

### 4. Memory store: SQLite via `rusqlite`

Per-ticket memory needs to persist across invocations, be queryable by ticket ID, and be lightweight. SQLite fits all three. A single table: `memory(ticket_id TEXT PRIMARY KEY, context TEXT, updated_at TEXT)`.

Alternative considered: flat JSON files per ticket — simpler, but harder to query and no atomic writes.

Memory lives at `~/.orga/memory.db` by default, overridable in config.

### 5. Config format: TOML via `toml` + `serde`

TOML is human-writable, unambiguous, and idiomatic for Rust tooling (Cargo uses it). Config is loaded at startup and passed to the backend constructor.

```toml
[agent]
name = "agent-1"

[board]
id = "board-xyz"
backend = "trello"

[trello]
api_key = "..."
token = "..."
member_id = "abc123"

[memory]
path = "~/.orga/memory.db"
```

`[agent]` captures backend-agnostic identity. Each backend config block carries its own identity field (e.g., `member_id` for Trello, `account_id` for a future Jira backend). Only the active backend's identity field is used.

### 6. Output: human-readable default, `--json` flag

All read commands (`ticket list`, `ticket show`, `memory get`) print formatted text by default and structured JSON with `--json`. Write commands (`comment`, `assign`, `move`, etc.) print a success line or error. This makes the CLI useful both for humans debugging and for agent skills parsing output.

## Risks / Trade-offs

- **Trello API rate limits** → Mitigation: all calls are on-demand (no polling loop), so rate limits are unlikely to be hit in normal agent skill usage.
- **Trello credential exposure in config file** → Mitigation: document that the config file should have restricted permissions (`chmod 600`); future work could add keychain/env var support.
- **SQLite memory db permissions** → Mitigation: default path in user home dir avoids permission issues.
- **Blocking HTTP calls** → Accepted trade-off: `orga` is a short-lived CLI process, async is unnecessary complexity. `reqwest` blocking client is appropriate.
- **Trello's card hierarchy is shallow** (no native sub-cards) → Mitigation: sub-tickets are implemented as new cards in the same board with a naming convention and a checklist link on the parent, or using Trello's built-in card attachments. This needs validation against the Trello API.

## Open Questions

- Does Trello support parent/child card relationships natively, or do we use a naming convention + checklist item with a card link?
- Should `orga ticket list` show only cards in active lists (e.g., Todo, In Progress) or all assigned cards regardless of list?
- Should memory have a TTL or max size, or is that left to the operator to manage?
