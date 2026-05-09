# Example HEARTBEAT.md for an orga agent

```md
tasks:

- name: orga-ticket-check
  interval: 30m
  prompt: >
    Run `orga whoami --json` then `orga ticket list --json`.
    Find the first ticket where the latest comment is not from you.
    If found, load memory (`orga memory get <id>`), show the full ticket
    (`orga ticket show --json <id>`), do the work, save findings to memory,
    and post a comment with your findings or next question.
    If all tickets have your username as the latest commenter, reply HEARTBEAT_OK.

# Rules

- Always use --json when calling orga.
- Work one ticket per heartbeat — the first where the latest comment is not yours.
- Never move a ticket without explicit human confirmation in a comment.
- If nothing needs attention, reply HEARTBEAT_OK.
```
