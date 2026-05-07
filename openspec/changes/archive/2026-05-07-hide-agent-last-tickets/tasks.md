## 1. Data Model

- [x] 1.1 Add `last_commenter_is_agent: bool` field to `TicketSummary` in `src/models.rs`

## 2. Trello Backend

- [x] 2.1 Update `list_assigned` to request `actions=commentCard,createCard` (was `createCard` only)
- [x] 2.2 In `card_to_summary`, find the latest `commentCard` action by date, call `parse_agent_tag` on its text, and set `last_commenter_is_agent` accordingly (default `false` when no comment actions present)

## 3. CLI Filtering

- [x] 3.1 In `main.rs`, apply agent-last filter in the default `ticket list` path: exclude tickets where `last_commenter_is_agent == true` (before the existing `completed`/`all` checks)

## 4. Tests

- [x] 4.1 Add unit test: `card_to_summary` sets `last_commenter_is_agent: true` when latest comment action has an agent tag
- [x] 4.2 Add unit test: `card_to_summary` sets `last_commenter_is_agent: false` when latest comment has no agent tag
- [x] 4.3 Add unit test: `card_to_summary` sets `last_commenter_is_agent: false` when there are no comment actions
- [x] 4.4 Verify existing `ticket list` JSON output tests include the new field
