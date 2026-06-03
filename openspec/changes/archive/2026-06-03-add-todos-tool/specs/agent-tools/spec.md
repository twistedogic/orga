## MODIFIED Requirements

### Requirement: Tool set
The agent loop SHALL expose the following tools to the LLM during a ticket cycle. Each tool SHALL correspond to an existing orga operation. In dry-run mode, mutating tools SHALL be logged but not executed; read tools SHALL execute normally. When subagents are configured, the main agent receives a narrowed tool set; subagents receive the tool set defined in their config plus `return` and `todos`. When no subagents are configured, the full tool set plus `todos` is exposed unchanged. `todos` SHALL always be available to all agents regardless of config.

| Tool | Available to | Mutating | Maps to |
|------|-------------|----------|---------|
| `comment(text)` | main agent | yes | `board.comment()` |
| `assign(username)` | subagent (if configured) | yes | `board.assign()` |
| `create_sub(title)` | subagent (if configured) | yes | `board.create_sub()` |
| `set_memory(context)` | main agent + subagent (if configured) | yes | `memory_store.set()` |
| `compact(summary)` | main agent + subagent (if configured) | yes | `compaction_store.set()` |
| `dispatch(subagent, task)` | main agent | yes | runs subagent loop |
| `return(result)` | subagent | no | terminates subagent loop |
| `done(comment?)` | main agent | yes | `board.return_ticket()` |
| `skip()` | main agent | no | ends cycle, no board action |
| `todos(todos)` | all agents (always) | no | `memory_store` scoped key |

#### Scenario: todos always available to main agent
- **WHEN** the main agent loop runs (with or without subagents configured)
- **THEN** `todos` is present in the tool definitions regardless of config

#### Scenario: todos always available to subagent
- **WHEN** a subagent loop runs
- **THEN** `todos` is present in the tool definitions regardless of the subagent's `tools` config

#### Scenario: Main agent has narrowed tool set
- **WHEN** subagents are configured and the main agent loop runs
- **THEN** the LLM only sees `comment`, `dispatch`, `skip`, `done`, `set_memory`, `compact`, `todos` in tool definitions

#### Scenario: Full tool set when no subagents configured
- **WHEN** no subagents are configured
- **THEN** the full tool set including `todos` is exposed to the LLM

#### Scenario: comment tool posts to board
- **WHEN** the LLM calls `comment(text: "update: work started")`
- **THEN** `board.comment(ticket_id, text)` is called and the result is returned to the LLM

#### Scenario: done tool returns ticket
- **WHEN** the LLM calls `done(comment: "work complete")`
- **THEN** `board.return_ticket(ticket_id, Some("work complete"))` is called and the cycle ends

#### Scenario: done tool without comment
- **WHEN** the LLM calls `done()` with no comment
- **THEN** `board.return_ticket(ticket_id, None)` is called
