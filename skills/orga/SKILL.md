---
name: orga
description: Use when working tickets on an orga kanban board — checking assigned tickets, investigating, commenting, moving tickets, or managing per-ticket memory.
---

# orga

`orga` is a kanban board CLI for agents. Use it to receive work, communicate with humans, and advance tickets through the board.

## Session start (always do this first)

```bash
orga whoami --json          # learn your own @username for this session
orga ticket list --json     # get all assigned, non-completed tickets
```

Find the first ticket where the latest comment was NOT posted by you. That is the ticket to work this session. If all tickets have your username as the latest commenter, stop — you are waiting for human responses.

```bash
orga ticket show --json <id>   # load full ticket: description, comments, checklists
```

Check `comments[-1].who.username` against your username from `whoami`.

## Working a ticket

### Load memory first

```bash
orga memory get <id>
```

This may contain prior research, findings, or context from previous sessions. Always read it before starting work.

### Save findings

After investigating, save anything useful for future sessions:

```bash
orga memory set <id> "<context, research results, key findings>"
```

Memory overwrites on each `set` — include all relevant context, not just new findings.

### Communicate via comments

Comments are the only way to communicate with humans. Use them to:
- Ask clarifying questions
- Request decisions
- Report what you found
- Explain what you need before you can proceed

```bash
orga ticket comment <id> "<text>"
```

After commenting, **stop and wait**. Do not comment multiple times in the same session without new information. The human will respond in their next turn.

### If the ticket is wrong or out of scope

Return it to the creator with an explanation:

```bash
orga ticket return <id> --comment "<reason>"
```

## Finishing a ticket

Only move a ticket after a human has explicitly confirmed in a comment that the work is done.

First discover the available columns:

```bash
orga columns --json
```

Then move to the appropriate target column:

```bash
orga ticket move <id> "<column name>"
```

## Key rules

- Always use `--json` — every command supports it, parse that not human-readable output
- Always call `whoami` at session start — never assume your username
- Work one ticket per session — the first eligible one (latest comment not from you)
- Read memory before working, write memory after findings
- Comments are your only communication channel — use them deliberately
- Never move a ticket without explicit human confirmation
- Use `ticket return` when the ticket is misrouted or out of scope, not when done

## Command reference

```bash
orga whoami --json
orga ticket list --json                          # non-completed, assigned to you
orga ticket list --json --completed              # completed tickets
orga ticket list --json --all                    # all tickets
orga ticket show --json <id>
orga ticket comment <id> "<text>"
orga ticket move <id> "<list name>"
orga ticket return <id> [--comment "<text>"]
orga ticket assign <id> <username>
orga ticket create-sub <parent_id> "<title>"
orga checklist add <ticket_id> "<text>"
orga checklist check <ticket_id> <item_id>
orga memory get <ticket_id>
orga memory set <ticket_id> "<context>"
orga columns --json
```

## Config

Config lives at `~/.orga/config.toml` (or `$ORGA_CONFIG`):

```toml
[agent]
name = "agent-1"

[board]
id = "board-xyz"
backend = "trello"

[trello]
api_key = "..."
token = "..."
member_id = "..."

[memory]          # optional
path = "/path/to/memory.db"   # default: ~/.orga/memory.db
```
