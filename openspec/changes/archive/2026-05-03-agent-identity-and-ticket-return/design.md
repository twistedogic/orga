## Context

`orga` is a stateless Rust CLI for LLM agents to interact with Trello boards. Agents share a Trello API account with their human operators. Currently there is no way for an agent to:

1. Know its own Trello identity (to filter its own comments from `ticket show` output)
2. Know who created a ticket (to reassign when blocked or done)
3. Mark comments as agent-authored when multiple agents or humans share the same Trello account

The `get_ticket` path already fetches Trello actions (for `commentCard`) so the `createCard` action can be included at zero extra cost. The `Board` trait is the single extension point for new capabilities.

## Goals / Non-Goals

**Goals:**
- `orga whoami` — resolve agent's Trello profile (id, username, full_name) from config `member_id`
- `Ticket.creator` — always populated on `ticket show` from Trello card action history
- Agent comment tagging — append `\n\n_[orga:agent-1]_` to every comment posted by the CLI; parse `agent_name` back out on read
- `orga ticket return <id> [--comment <text>]` — post optional comment then reassign to creator

**Non-Goals:**
- Differentiating multiple humans sharing an account (only agent vs. human distinction)
- Modifying `ticket list` to include creator (extra API call per card, too expensive)
- Any authentication changes

## Decisions

### Creator fetch: extend existing actions query, not a separate request

`get_ticket` already calls `/1/cards/{id}?actions=commentCard`. Extending to `actions=commentCard,createCard` costs nothing extra. The `createCard` action's `memberCreator` is the card creator.

Alternative considered: separate `/1/cards/{id}/actions?filter=createCard` call. Rejected — adds a round-trip with no benefit.

### Agent comment tag format: italic footer `\n\n_[orga:<agent-name>]_`

The tag is appended to every comment posted via `orga`. On read, it is stripped from `content` and the agent name is surfaced as `Comment.agent_name: Option<String>`.

Format: `\n\n_[orga:agent-name]_` — renders as italic in Trello's Markdown, unambiguous to parse with a simple regex, human-readable in the UI.

Alternative considered: HTML comment `<!-- orga-agent: name -->`. Rejected — Trello may strip HTML comments or render them as raw text.

Alternative considered: JSON block at end. Rejected — ugly in Trello UI.

The tag is appended only when `agent.name` is set in config (always true for agents, optional for humans using the CLI directly).

### `ticket return` uses existing `assign` + `comment` primitives internally

`return_ticket` on the `Board` trait calls `get_ticket` to read `creator`, then optionally posts a comment (with agent tag), then calls `assign`. This keeps the implementation in one place and reuses proven code paths.

### `whoami` is always a live API call

`member_id` is in config but username and full_name are not. Since the purpose is to give agents a resolvable identity for display and for use in `ticket assign`, the live resolved profile is the right output. No caching — the CLI is stateless.

## Risks / Trade-offs

- [Old cards without a `createCard` action] → `creator` is `None`; `ticket return` returns an error: `"ticket has no known creator"`. Agent must handle this case.
- [Tag parsing coupled to format] → If the tag format ever changes, old tagged comments won't be re-parsed. Acceptable since this is an append-only tagging scheme with no migration needed.
- [Agent name contains special regex chars] → Agent names from config are escaped before being used in the strip regex.

## Migration Plan

No migration needed. Existing tickets gain `creator: null` until fetched fresh. Existing comments get `agent_name: null` (no tag present). All changes are additive to the JSON output.
