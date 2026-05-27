---
name: orga
description: Use when working tickets on an orga kanban board — checking assigned tickets, investigating, commenting, moving tickets, or managing per-ticket memory.
---

# orga

`orga` is a kanban board CLI for agents. Use it to receive work, communicate with humans, and advance tickets through the board.

## References

- [Agile Manifesto](references/agile-manifesto.md) — the values and twelve principles that guide agile development
- [Kanban Guide](references/kanban.md) — definition, three practices, flow metrics, and key terms
- [openclaw HEARTBEAT.md](references/openclaw-heartbeat.md) — example session script for an openclaw agent using orga
- [Zeroclaw SOP](references/zeroclaw-sop.md) — Standard Operating Procedures setup, event flow, and orga integration

## Session start (always do this first)

```bash
orga whoami --json          # learn your own @username for this session
orga ticket list --json     # get all assigned, non-completed tickets
```

Find the first ticket where the latest comment was NOT posted by you. That is the ticket to work this session. If all tickets have your username as the latest commenter, stop — you are waiting for human responses.

```bash
orga ticket show --json <id>   # load full ticket: description, sub_tickets, comments
```

Check `comments[-1].who.username` against your username from `whoami`.

## Decomposing work

When a ticket is too broad or has multiple independent deliverables, break it into sub-tickets.

**When to decompose:**
- The ticket contains multiple independent tasks that can be worked separately
- Work needs to be tracked at a finer grain than the parent ticket
- You want to hand off a piece of work to another agent or human

**How to create sub-tickets:**

```bash
orga ticket create-sub <parent_id> "<title>"
orga ticket create-sub <parent_id> "<title>" --description "<details>"
orga ticket create-sub <parent_id> "<title>" --list "<column name>"
```

- Sub-tickets are **unassigned by default** — the human decides who picks them up
- `--list` defaults to the parent's current list if omitted
- After creating, use `--json` to capture the sub-ticket ID(s)

**After creating sub-tickets (always do this):**

1. Comment on the parent ticket summarizing the decomposition:
   ```bash
   orga ticket comment <parent_id> "Decomposed into sub-tickets: [#<id1> <title1>], [#<id2> <title2>]"
   ```
2. Stop and wait — do not continue working. The human will assign and prioritize the sub-tickets.

**Exploring sub-tickets:**

Sub-tickets appear in `ticket show --json` under the `sub_tickets` field on Linear.
On Trello, `sub_tickets` is always empty — use the sub-ticket IDs from `create-sub` output.

```bash
orga ticket show --json <sub_ticket_id>   # load a specific sub-ticket
```

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

## Artifacts

Artifacts are named blobs (files, reports, diffs, etc.) you produce while working a ticket. They are stored in a git-backed artifact store and scoped to the current agent.

```bash
orga artifact commit <ticket_id> <name> [content]   # store inline text as artifact
orga artifact commit <ticket_id> <name> --file <path>  # store file contents
orga artifact list  <ticket_id>                      # list all agents' artifacts for ticket
orga artifact get   <ticket_id> <name>               # retrieve your artifact by name
```

- `commit` accepts either inline `content` or `--file <path>` (mutually exclusive).
- `list` shows artifacts from **all** agents for the ticket (`agent/name\ttimestamp`).
- `get` is scoped to the **current agent** — use it to retrieve artifacts you previously committed.
- All commands support `--json`.

Use artifacts to persist structured outputs (reports, diffs, data files) that are too large or too structured for ticket memory.

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
orga ticket create-sub <parent_id> "<title>" [--description "<text>"] [--list "<column name>"]
orga memory get <ticket_id>
orga memory set <ticket_id> "<context>"
orga columns --json
orga artifact commit <ticket_id> <name> [content] [--file <path>]
orga artifact list  <ticket_id>
orga artifact get   <ticket_id> <name>
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

[artifact]        # optional — required for orga artifact commands
backend = "git"

[artifact.git]
path = "/path/to/artifact-repo"   # required: local git repo path
remote = "origin"                 # optional: push after each commit
branch = "main"                   # optional: branch to push to
ssh_key = "/path/to/key"          # optional: SSH key for auth
ssh_passphrase = "..."            # optional
http_username = "..."             # optional: HTTP basic auth
http_password = "..."             # optional
```
