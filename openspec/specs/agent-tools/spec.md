# agent-tools Specification

## Purpose
Typed tool set exposed to the LLM during a ticket cycle. Each tool maps to an existing orga board, memory, or artifact operation. Dry-run mode suppresses all mutating tools while allowing read tools to execute.

## Requirements
### Requirement: Tool set
The agent loop SHALL expose the following tools to the LLM during a ticket cycle. Each tool SHALL correspond to an existing orga operation. In dry-run mode, mutating tools SHALL be logged but not executed; read tools SHALL execute normally. When subagents are configured, the main agent receives a narrowed tool set; subagents receive the tool set defined in their config plus `return`. When no subagents are configured, the full tool set is exposed unchanged.

| Tool | Available to | Mutating | Maps to |
|------|-------------|----------|---------|
| `comment(text)` | main agent | yes | `board.comment()` |
| `move_ticket(list)` | subagent (if configured) | yes | `board.move_ticket()` |
| `assign(username)` | subagent (if configured) | yes | `board.assign()` |
| `create_sub(title)` | subagent (if configured) | yes | `board.create_sub()` |
| `set_memory(context)` | main agent + subagent (if configured) | yes | `memory_store.set()` |
| `commit_artifact(name, content)` | subagent (if configured) | yes | `artifact_store.commit()` |
| `get_artifact(name)` | subagent (if configured) | no | `artifact_store.get()` |
| `compact(summary)` | main agent + subagent (if configured) | yes | `compaction_store.set()` |
| `dispatch(subagent, task)` | main agent | yes | runs subagent loop |
| `return(result)` | subagent | no | terminates subagent loop |
| `done(comment?)` | main agent | yes | `board.return_ticket()` |
| `skip()` | main agent | no | ends cycle, no board action |

#### Scenario: Main agent has narrowed tool set
- **WHEN** subagents are configured and the main agent loop runs
- **THEN** the LLM only sees `comment`, `dispatch`, `skip`, `done`, `set_memory`, `compact` in tool definitions

#### Scenario: Subagent tool set from config
- **WHEN** a subagent with `tools = ["get_artifact", "commit_artifact"]` runs its loop
- **THEN** the LLM only sees `get_artifact`, `commit_artifact`, and `return` in tool definitions

#### Scenario: Full tool set when no subagents configured
- **WHEN** no subagents are configured
- **THEN** the full tool set (all original tools) is exposed to the LLM unchanged

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

### Requirement: dispatch tool
The main agent SHALL have access to a `dispatch(subagent, task)` tool when subagents are configured. It is a mutating tool. In dry-run mode it SHALL be logged but not executed.

#### Scenario: dispatch tool available when subagents configured
- **WHEN** subagents are configured and the main agent loop runs
- **THEN** `dispatch` is included in the tool definitions sent to the LLM

#### Scenario: dispatch suppressed in dry-run
- **WHEN** dry-run is active and the main agent calls `dispatch(subagent: "researcher", task: "...")`
- **THEN** the action is logged but the subagent loop is not started; a dry-run notice is returned as the tool result

### Requirement: return tool
The subagent tool set SHALL include a `return(result)` terminal tool. Calling it ends the subagent loop and surfaces the result to the main agent.

#### Scenario: return tool available in subagent loop
- **WHEN** a subagent loop runs
- **THEN** `return` is included in the tool definitions sent to the LLM

#### Scenario: return is a terminal tool
- **WHEN** the subagent calls `return(result: "...")`
- **THEN** the subagent loop terminates immediately after processing this tool call

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

### Requirement: File tools
The agent loop SHALL expose `read_file`, `write_file`, and `list_files` tools when `[workspace]` is configured. These tools SHALL be available to subagents via their `tools` list. In dry-run mode, `write_file` SHALL be logged but not executed; `read_file` and `list_files` SHALL execute normally.

| Tool | Mutating | Maps to |
|------|----------|---------|
| `read_file(path)` | no | `workspace_store.read()` |
| `write_file(path, content)` | yes | `workspace_store.write()` |
| `list_files()` | no | `workspace_store.list()` |

#### Scenario: read_file executes in dry-run
- **WHEN** dry-run is active and the LLM calls `read_file(path: "notes.md")`
- **THEN** the file is read and its content returned (read tools are not suppressed)

#### Scenario: write_file suppressed in dry-run
- **WHEN** dry-run is active and the LLM calls `write_file(path: "out.md", content: "...")`
- **THEN** the action is logged but not executed; a dry-run notice is returned as the tool result

#### Scenario: list_files executes in dry-run
- **WHEN** dry-run is active and the LLM calls `list_files()`
- **THEN** the file listing is returned normally (read tools are not suppressed)
