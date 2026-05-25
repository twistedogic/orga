# agent-tools Specification

## Purpose
Typed tool set exposed to the LLM during a ticket cycle. Each tool maps to an existing orga board, memory, or artifact operation. Dry-run mode suppresses all mutating tools while allowing read tools to execute.

## Requirements
### Requirement: Tool set
The agent loop SHALL expose the following tools to the LLM during a ticket cycle. Each tool SHALL correspond to an existing orga operation. In dry-run mode, mutating tools SHALL be logged but not executed; read tools SHALL execute normally.

| Tool | Mutating | Maps to |
|------|----------|---------|
| `comment(text)` | yes | `board.comment()` |
| `move_ticket(list)` | yes | `board.move_ticket()` |
| `assign(username)` | yes | `board.assign()` |
| `create_sub(title)` | yes | `board.create_sub()` |
| `add_checklist_item(text)` | yes | `board.add_checklist_item()` |
| `check_item(item_id)` | yes | `board.check_item()` |
| `set_memory(context)` | yes | `memory_store.set()` |
| `commit_artifact(name, content)` | yes | `artifact_store.commit()` |
| `get_artifact(name)` | no | `artifact_store.get()` |
| `compact(summary)` | yes | `compaction_store.set()` |
| `done(comment?)` | yes | `board.return_ticket()` |
| `skip()` | no | ends cycle, no board action |

#### Scenario: comment tool posts to board
- **WHEN** the LLM calls `comment(text: "update: work started")`
- **THEN** `board.comment(ticket_id, text)` is called and the result is returned to the LLM

#### Scenario: done tool returns ticket
- **WHEN** the LLM calls `done(comment: "work complete, see artifact report.md")`
- **THEN** `board.return_ticket(ticket_id, Some("work complete, see artifact report.md"))` is called and the cycle ends

#### Scenario: done tool without comment
- **WHEN** the LLM calls `done()` with no comment
- **THEN** `board.return_ticket(ticket_id, None)` is called

#### Scenario: skip tool ends cycle silently
- **WHEN** the LLM calls `skip()`
- **THEN** no board mutation occurs and the cycle ends

#### Scenario: get_artifact executes in dry-run
- **WHEN** dry-run is active and the LLM calls `get_artifact(name: "report.md")`
- **THEN** the artifact is fetched and returned to the LLM (read tools are not suppressed)

#### Scenario: commit_artifact suppressed in dry-run
- **WHEN** dry-run is active and the LLM calls `commit_artifact(name: "report.md", content: "...")`
- **THEN** the action is logged but not executed; a dry-run notice is returned as the tool result

### Requirement: Tool error handling
If a tool call fails (e.g., invalid ticket ID, network error, artifact not found), the error SHALL be returned as a `tool_result` with `is_error: true`. The cycle SHALL continue within the cap.

#### Scenario: Tool error returned to LLM
- **WHEN** the LLM calls `move_ticket(list: "Nonexistent Column")` and the board returns an error
- **THEN** the error message is returned as a tool_result and the LLM receives it in the next turn

### Requirement: Ticket context construction
Before the first LLM turn, the loop SHALL construct the ticket context as follows:
- System prompt: workflow prompt for the ticket's current column (if configured) + agent identity
- User message: ticket fields (id, title, description, list, url, assignees, creator), checklists, comments (with compaction applied if present), current memory (if any), artifact list with inline content up to `max_artifact_inline_bytes` per artifact (metadata only above the cap)

#### Scenario: Workflow prompt injected
- **WHEN** a workflow entry matches the ticket's column
- **THEN** the system prompt includes the workflow prompt text

#### Scenario: Artifact content inlined below cap
- **WHEN** an artifact's content is below `max_artifact_inline_bytes`
- **THEN** its full content is included in the user message

#### Scenario: Artifact metadata only above cap
- **WHEN** an artifact's content exceeds `max_artifact_inline_bytes`
- **THEN** only its metadata (name, size, committed_at) is included; the LLM may call `get_artifact` to retrieve it
