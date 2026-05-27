## Why

The `create-sub` command only accepts a title, giving agents no way to provide description or initial placement when decomposing work. Checklists are used internally to surface sub-tickets in `ticket show`, but this conflates two different concepts and loses structured data (URL, state) that agents need to navigate sub-tickets. These gaps make autonomous decomposition workflows incomplete.

## What Changes

- `orga ticket create-sub` gains `--description` and `--list` optional flags; sub-ticket is created unassigned and placed in the parent's list by default
- `orga ticket show` now includes a `sub_tickets` field (array of `TicketSummary`) in JSON output
- **BREAKING** `Ticket` model: `checklists: Vec<Checklist>` replaced by `sub_tickets: Vec<TicketSummary>`
- **BREAKING** `Checklist` and `ChecklistItem` types removed from models
- **BREAKING** `Board` trait: `add_checklist_item` and `check_item` methods removed; `create_sub` signature updated to accept optional description and list
- **BREAKING** `orga checklist add` and `orga checklist check` commands removed
- Orga agent skill updated with sub-ticket decomposition guidance and updated command reference

## Capabilities

### New Capabilities

- `subticket-create`: Create sub-tickets with optional description and list placement, unassigned by default
- `subticket-show`: `ticket show` exposes sub-tickets as a structured `sub_tickets` array in JSON output

### Modified Capabilities

- `cli-commands`: Checklist commands removed; `create-sub` gains new optional flags
- `board-abstraction`: `Board` trait updated — checklist methods removed, `create_sub` signature changed

## Impact

- `src/models.rs` — remove `Checklist`, `ChecklistItem`; add `sub_tickets` to `Ticket`
- `src/main.rs` — remove `checklist` subcommand; add `--description` and `--list` to `create-sub`
- `src/board/mod.rs` — update `Board` trait
- `src/board/linear.rs` — map `children.nodes` to `Vec<TicketSummary>`; update `create_sub` to accept description + list (state lookup by name)
- `src/board/trello.rs` — update `create_sub` to accept description + list; `sub_tickets: vec![]` always
- `~/.agents/skills/orga/SKILL.md` — add decomposition workflow section, remove checklist section, update command reference
