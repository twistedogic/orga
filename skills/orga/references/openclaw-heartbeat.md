# Example HEARTBEAT.md for an orga agent

```md
tasks:

- name: orga-ticket-check
  interval: 30m
  prompt: >
    Run `orga whoami --json` then `orga ticket list --json`.
    Find the first ticket where the latest comment is not from you.
    If found, scan the context repository (`orga memory list --json`), read any
    relevant topic files (`orga memory read <path>`), show the full ticket
    (`orga ticket show --json <id>`), do the work, and post a comment with your
    findings or next question. If you discover cross-ticket-valuable knowledge
    (patterns, conventions, architectural decisions), write it to the context
    repository (`orga memory write <path> "<content>"`).
    If all tickets have your username as the latest commenter, reply HEARTBEAT_OK.

- name: orga-memory-reflect
  interval: 6h
  prompt: >
    Review the context repository (`orga memory list --json`) and recent completed
    tickets. Identify any learnings not yet captured as topic files. Write new or
    updated topic files using `orga memory write`. Then check if the repository
    has grown large or disorganized — if there are many overlapping files or files
    covering multiple unrelated topics, run `orga memory defrag` to reorganize.

# Rules

- Always use --json when calling orga.
- Work one ticket per heartbeat — the first where the latest comment is not yours.
- Scan the context repository at the start of each ticket session before starting work.
- Write to memory only for cross-ticket-valuable insights, not ticket-specific facts.
- The sleep-time agent runs automatically after done() — you don't need to reflect manually every session.
- Never move a ticket without explicit human confirmation in a comment.
- If nothing needs attention, reply HEARTBEAT_OK.
```
