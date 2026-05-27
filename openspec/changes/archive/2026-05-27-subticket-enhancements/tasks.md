## 1. Model Changes

- [x] 1.1 Remove `Checklist` and `ChecklistItem` structs from `src/models.rs`
- [x] 1.2 Add `sub_tickets: Vec<TicketSummary>` field to `Ticket` struct in `src/models.rs`
- [x] 1.3 Fix all compile errors caused by removed types (imports, usages)

## 2. Board Trait

- [x] 2.1 Remove `add_checklist_item` and `check_item` from the `Board` trait in `src/board/mod.rs`
- [x] 2.2 Update `create_sub` signature to `fn create_sub(&self, parent_id: &str, title: &str, description: Option<&str>, list: Option<&str>) -> Result<Ticket, OrgaError>`

## 3. Linear Backend

- [x] 3.1 Update `linear_issue_to_ticket` to map `children.nodes` to `Vec<TicketSummary>` (id, title, url, list_name, completed) instead of a `Checklist`
- [x] 3.2 Update `create_sub` to accept `description: Option<&str>` and `list: Option<&str>`
- [x] 3.3 When `list` is `Some`, resolve state name → `stateId` via `team_states()`; error if not found
- [x] 3.4 When `list` is `None`, use the parent's current state as the default
- [x] 3.5 Pass `description` and `stateId` into the `issueCreate` GraphQL mutation
- [x] 3.6 Remove `create_sub_issue` private helper or update it to accept the new params
- [x] 3.7 Remove `add_checklist_item` and `check_item` implementations

## 4. Trello Backend

- [x] 4.1 Update `create_sub` to accept `description: Option<&str>` and `list: Option<&str>`
- [x] 4.2 When `list` is `Some`, resolve list name → list id via `list_columns()`; error if not found
- [x] 4.3 When `list` is `None`, use the parent's `list_id` as before
- [x] 4.4 Pass `desc` param when `description` is `Some`
- [x] 4.5 Set `sub_tickets: vec![]` in `get_ticket` response
- [x] 4.6 Remove `add_checklist_item` and `check_item` implementations

## 5. CLI Commands

- [x] 5.1 Remove the `checklist` subcommand group from `src/main.rs` (both `add` and `check` variants)
- [x] 5.2 Add `--description: Option<String>` and `--list: Option<String>` args to `CreateSub` in `src/main.rs`
- [x] 5.3 Pass the new args through to `board.create_sub()`

## 6. Tests

- [x] 6.1 Update existing Linear `get_ticket` tests to use `sub_tickets` instead of `checklists`
- [x] 6.2 Add Linear `create_sub` test: with description and list
- [x] 6.3 Verify Trello `create_sub` compiles and `sub_tickets` is empty in result

## 7. Skill Update

- [x] 7.1 Add "Decomposing work" section to `~/.agents/skills/orga/SKILL.md` — when to use `create-sub`, post-creation workflow (comment on parent with sub-ticket links, stop and wait)
- [x] 7.2 Remove checklist section from SKILL.md
- [x] 7.3 Update command reference in SKILL.md — add `--description`/`--list` to `create-sub`, remove `orga checklist` commands
