## ADDED Requirements

### Requirement: dispatch tool
The agent tool set SHALL include a `dispatch(subagent, task)` tool available to the main agent when subagents are configured. It is a mutating tool. In dry-run mode it SHALL be logged but not executed.

| Tool | Mutating | Maps to |
|------|----------|---------|
| `dispatch(subagent, task)` | yes | runs subagent loop, returns result string |

#### Scenario: dispatch tool available when subagents configured
- **WHEN** subagents are configured and the main agent loop runs
- **THEN** `dispatch` is included in the tool definitions sent to the LLM

#### Scenario: dispatch suppressed in dry-run
- **WHEN** dry-run is active and the main agent calls `dispatch(subagent: "researcher", task: "...")`
- **THEN** the action is logged but the subagent loop is not started; a dry-run notice is returned as the tool result

### Requirement: return tool
The subagent tool set SHALL include a `return(result)` terminal tool. It is not mutating. Calling it ends the subagent loop and surfaces the result.

| Tool | Mutating | Maps to |
|------|----------|---------|
| `return(result)` | no | terminates subagent loop, returns result to main agent |

#### Scenario: return tool available in subagent loop
- **WHEN** a subagent loop runs
- **THEN** `return` is included in the tool definitions sent to the LLM

#### Scenario: return is a terminal tool
- **WHEN** the subagent calls `return(result: "...")`
- **THEN** the subagent loop terminates immediately after processing this tool call

## MODIFIED Requirements

### Requirement: Tool set
When subagents are configured, the main agent loop SHALL expose a narrowed tool set: `comment`, `dispatch`, `skip`, `done`. Subagents receive the tool set defined in their config entry plus `return`. When no subagents are configured, the full existing tool set applies unchanged.

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
- **THEN** the LLM only sees `comment`, `dispatch`, `skip`, `done` in tool definitions

#### Scenario: Subagent tool set from config
- **WHEN** a subagent with `tools = ["get_artifact", "commit_artifact"]` runs its loop
- **THEN** the LLM only sees `get_artifact`, `commit_artifact`, and `return` in tool definitions

#### Scenario: Full tool set when no subagents configured
- **WHEN** no subagents are configured
- **THEN** the full tool set (all original tools) is exposed to the LLM unchanged
