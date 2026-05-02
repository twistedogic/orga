## Context

The `Board` trait is the backend-agnostic interface all CLI commands dispatch through. Trello already exposes a `GET /boards/{id}/lists` endpoint, and `TrelloBackend` has a private `board_lists()` helper that calls it. The CLI has no way to surface that data today.

The `Column` type needs to be added to `models.rs` as a shared, serializable struct — consistent with how `Ticket`, `Member`, and `Checklist` are already defined.

## Goals / Non-Goals

**Goals:**
- Expose board columns (id + name) via `orga columns`
- Add `list_columns()` to the `Board` trait so any future backend must implement it
- Reuse the existing `board_lists()` private method in `TrelloBackend`

**Non-Goals:**
- Filtering, ordering, or paginating columns
- Creating or archiving columns
- Caching — each invocation hits the API

## Decisions

### `Column` lives in `models.rs`

Consistent with all other shared types (`Ticket`, `Member`, etc.). Backend-specific Trello structs (`TrelloList`) remain private in `trello.rs` and are mapped to `Column` at the boundary.

### `list_columns()` on the `Board` trait (not a standalone function)

Alternatives considered:
- A free function calling Trello directly — violates the backend abstraction
- A sub-command under `ticket` — semantically wrong, columns are a board property

Adding it to the trait keeps the CLI handler backend-agnostic and ensures future backends must implement it.

### `orga columns` as a top-level command (not `orga board columns`)

No `board` subcommand group exists today. Adding one for a single command creates unnecessary nesting. A flat top-level command is consistent with `orga init`.

### Output format

Human-readable: `<id>  <name>` (tab-separated), one per line — easy to parse with awk/cut by agents.  
JSON: array of `{id, name}` objects — matches the pattern of all other `--json` outputs.

## Risks / Trade-offs

- [Trello API rate limits] → No mitigation; consistent with all other commands. Caching is a future concern.
- [Trait change requires all `Board` implementations to add `list_columns()`] → Acceptable: only one implementation exists (`TrelloBackend`).
