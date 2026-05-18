## Why

orga currently only supports Trello. Adding Linear as a backend broadens adoption to teams that use Linear, without changing any CLI command or agent skill — the `Board` trait abstraction means the agent doesn't know or care which backend is active.

## What Changes

- New `LinearBackend` struct implementing the `Board` trait via Linear's GraphQL API
- New `[linear]` config section (`api_key` only — viewer identity is auto-resolved from the token)
- `board.backend = "linear"` becomes a valid config value
- `board.id` maps to a Linear **team** ID (workflow states live on teams)
- Checklist operations (`add_checklist_item`, `check_item`) map to sub-issues — **checklist items and sub-issues are unified in Linear**
- `get_ticket` returns sub-issues as a synthetic checklist in the `checklists` field
- `create_sub` and `add_checklist_item` both create Linear sub-issues (same underlying mutation, different return shapes)
- Agent comment tagging (`_[orga:name]_`) works unchanged — Linear renders markdown

## Capabilities

### New Capabilities

- `linear-backend`: Full `Board` trait implementation backed by Linear's GraphQL API, including ticket read/write, comments, column management, sub-issues as checklists, and viewer identity resolution

### Modified Capabilities

- `board-abstraction`: `build_board` factory and config validation must recognize `"linear"` as a supported backend; `config.rs` gains a `[linear]` section

## Impact

- New file: `src/board/linear.rs`
- Modified: `src/board/mod.rs` — register Linear in `build_board`
- Modified: `src/config.rs` — add `LinearConfig`, extend `AppConfig`, update `validate()`
- New dependency: none expected — `reqwest` (blocking) already available; GraphQL is just a POST with JSON body
- No changes to CLI commands, models, or agent output format
