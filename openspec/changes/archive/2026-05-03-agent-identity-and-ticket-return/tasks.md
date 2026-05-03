## 1. Models

- [x] 1.1 Add `creator: Option<Member>` field to `Ticket` in `src/models.rs`
- [x] 1.2 Add `agent_name: Option<String>` field to `Comment` in `src/models.rs`

## 2. Board Trait

- [x] 2.1 Add `whoami(&self) -> Result<Member, OrgaError>` to the `Board` trait in `src/board/mod.rs`
- [x] 2.2 Add `return_ticket(&self, id: &str, comment: Option<&str>) -> Result<(), OrgaError>` to the `Board` trait in `src/board/mod.rs`

## 3. Trello Backend

- [x] 3.1 Add `createCard` to the `actions` query param in `get_ticket` so the action is returned alongside comments
- [x] 3.2 Add `TrelloMember`-based creator extraction from the `createCard` action in `card_to_ticket`; populate `Ticket.creator`
- [x] 3.3 Implement `whoami` on `TrelloBackend` — call `GET /1/members/{member_id}?fields=id,username,fullName` and return a `Member`
- [x] 3.4 Implement `return_ticket` on `TrelloBackend` — fetch ticket, error if no creator, optionally post comment, then call `assign` with creator's username
- [x] 3.5 Add agent tag appending in `comment` method: when `agent.name` is available via the board (or pass it as a parameter), append `\n\n_[orga:<name>]_` to the text before posting
- [x] 3.6 Add agent tag parsing in `card_to_ticket` comment extraction: strip `\n\n_[orga:<name>]_` suffix from `content` and set `agent_name`

## 4. Config / Board Factory

- [x] 4.1 Thread `agent.name` into `TrelloBackend` so `comment` and `return_ticket` can use it for tagging — add `agent_name: Option<String>` field to `TrelloBackend` and pass it from `build_board`

## 5. CLI Commands

- [x] 5.1 Add `Commands::Whoami` variant to the `Commands` enum in `src/main.rs`
- [x] 5.2 Wire `Commands::Whoami` dispatch: call `board.whoami()`, print `@username (full_name)` or `{"id":…,"username":…,"full_name":…}` with `--json`
- [x] 5.3 Add `TicketCommands::Return { id, comment: Option<String> }` variant with `--comment` optional arg
- [x] 5.4 Wire `TicketCommands::Return` dispatch: call `board.return_ticket(&id, comment.as_deref())`, print success or `{"ok":true}`
- [x] 5.5 Update `print_ticket_detail` to show `Creator: @username` line when `ticket.creator` is `Some`

## 6. Tests

- [x] 6.1 Add integration test for `orga whoami --json` asserting `id`, `username`, `full_name` fields are present
- [x] 6.2 Add integration test for `orga ticket show <id> --json` asserting `creator` field is present (object or null)
- [x] 6.3 Add unit test for agent tag append logic (given agent name + text → expected tagged string)
- [x] 6.4 Add unit test for agent tag strip/parse logic (given tagged comment content → stripped content + agent_name)
- [x] 6.5 Add integration test for `orga ticket return <id>` success path
- [x] 6.6 Add integration test for `orga ticket return <id>` when no creator is present → non-zero exit + error message
