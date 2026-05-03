## Why

Agents operating on shared boards need to distinguish their own comments from human comments, even when they share the same API account. They also need a reliable, single-command way to return a ticket to its creator when blocked or when work is ready for human review.

## What Changes

- Add `orga whoami` command — resolves the agent's Trello identity (id, username, full_name) via the API
- Add `creator: Option<Member>` field to `Ticket` — always fetched from Trello action history on `ticket show`
- Tag agent-posted comments with `agent.name` from config so agents can distinguish their own comments from humans sharing the same Trello account
- Expose `agent_name: Option<String>` on `Comment` model — parsed from the tag on read, stripped from content
- Add `orga ticket return <id> [--comment <text>]` command — posts an optional comment then reassigns the ticket to its creator

## Capabilities

### New Capabilities

- `whoami`: `orga whoami` command that resolves the agent's Trello member profile from config `member_id`
- `ticket-creator`: Ticket always carries its creator, fetched from Trello card action history
- `agent-comment-tagging`: Agent comments are tagged with `agent.name`; `Comment` model exposes `agent_name` parsed from the tag
- `ticket-return`: `orga ticket return <id> [--comment <text>]` command that reassigns the ticket to its creator

### Modified Capabilities

- `cli-commands`: New top-level `whoami` command and new `ticket return` subcommand added
- `trello-backend`: Board trait gains `whoami`, creator fetch, and `return_ticket` methods

## Impact

- `src/models.rs` — `Ticket.creator: Option<Member>`, `Comment.agent_name: Option<String>`
- `src/board/mod.rs` — Board trait gains `whoami()` and `return_ticket(id, comment)`
- `src/board/trello.rs` — implements new trait methods; `get_ticket` fetches creator via action history; `comment` appends agent tag when `agent.name` is set
- `src/main.rs` — `Commands::Whoami` and `TicketCommands::Return` variants wired up
- `src/config.rs` — no structural changes; `agent.name` already present and used for tagging
