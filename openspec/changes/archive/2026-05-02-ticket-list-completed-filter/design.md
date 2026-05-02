## Context

`orga ticket list` currently calls Trello's `/members/{id}/cards` with `filter=open`, meaning closed (archived) cards are never returned. The `Ticket` model has no `completed` field, so the abstraction layer has no concept of ticket completion state. Agents have no way to query their history of completed work.

## Goals / Non-Goals

**Goals:**
- Add `completed: bool` to `Ticket` so the concept exists at the abstraction layer
- Allow `orga ticket list` to return completed, open, or all tickets via flags
- Keep the change minimal and non-breaking

**Non-Goals:**
- Defining "completed" as anything other than `card.closed` (e.g., column-name heuristics)
- Adding completion-state filtering to other commands (`ticket show`, `ticket move`, etc.)
- Exposing completion state mutation (agents never close tickets — existing constraint)

## Decisions

### `completed = card.closed` (not column-name heuristics)

Trello's `closed` field is a first-class boolean on every card. It maps cleanly to a single field with no configuration or naming conventions needed. Column-name heuristics (e.g., "Done") would require config, be fragile across boards, and leak Trello-specific concepts into the abstraction.

*Alternatives considered:* Column-name matching — rejected due to config burden and fragility.

### Fetch `filter=all` unconditionally in `list_assigned`

Rather than changing the `Board` trait signature to accept a filter parameter, the Trello backend always fetches all cards and the CLI filters client-side. This keeps the trait simple and the filtering concern at the CLI layer where flags live.

*Alternatives considered:* Passing a filter enum through the trait — rejected because it pushes a CLI concern into the backend abstraction and complicates future backends.

### `--completed` and `--all` flags (default: open only)

Default behaviour is unchanged (open tickets only) to avoid surprising existing callers. `--completed` shows only completed tickets; `--all` disables filtering entirely.

## Risks / Trade-offs

- **Slightly more data over the wire** — `filter=all` fetches closed cards that are usually discarded. Volume of assigned cards is small; not a practical concern.
- **`completed` field is additive to JSON output** — existing consumers parsing `--json` output may not expect the field, but adding a field is non-breaking for well-written consumers.

## Open Questions

None — decisions made during explore session.
