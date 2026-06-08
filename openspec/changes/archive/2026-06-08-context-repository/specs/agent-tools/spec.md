## REMOVED Requirements

### Requirement: set_memory tool in tool set
**Reason**: Replaced by `memory_write` (and the broader `memory-tools` capability). `set_memory` was ticket-scoped; `memory_write` is topic-scoped and writes to the git-backed context repository.
**Migration**: Agents use `memory_write(path, content, commit_msg)` instead of `set_memory(context)`.

## MODIFIED Requirements

### Requirement: Tool set
The agent loop SHALL expose the following tools to the LLM during a ticket cycle. Each tool SHALL correspond to an existing orga operation. In dry-run mode, mutating tools SHALL be logged but not executed; read tools SHALL execute normally. When subagents are configured, the main agent receives a narrowed tool set; subagents receive the tool set defined in their config plus `return` and `todos`. When no subagents are configured, the full tool set plus `todos` is exposed unchanged. `todos` SHALL always be available to all agents regardless of config. The four memory tools (`memory_list`, `memory_read`, `memory_write`, `memory_search`) SHALL always be available to all agents regardless of config.

| Tool | Available to | Mutating | Maps to |
|------|-------------|----------|---------|
| `comment(text)` | main agent | yes | `board.comment()` |
| `assign(username)` | subagent (if configured) | yes | `board.assign()` |
| `create_sub(title)` | subagent (if configured) | yes | `board.create_sub()` |
| `compact(summary)` | main agent + subagent (if configured) | yes | `compaction_store.set()` |
| `dispatch(subagent, task)` | main agent | yes | runs subagent loop |
| `return(result)` | subagent | no | terminates subagent loop |
| `done(comment?)` | main agent | yes | `board.return_ticket()` + triggers sleep-time agent |
| `skip()` | main agent | no | ends cycle, no board action |
| `todos(todos)` | all agents (always) | no | `TodoStore` scoped key |
| `memory_list()` | all agents (always) | no | `ContextRepository::list()` |
| `memory_read(path)` | all agents (always) | no | `ContextRepository::read()` |
| `memory_write(path, content, commit_msg)` | all agents (always) | yes | `ContextRepository::write()` |
| `memory_search(query)` | all agents (always) | no | `ContextRepository::search()` |

#### Scenario: todos always available to main agent
- **WHEN** the main agent loop runs (with or without subagents configured)
- **THEN** `todos` is present in the tool definitions regardless of config

#### Scenario: todos always available to subagent
- **WHEN** a subagent loop runs
- **THEN** `todos` is present in the tool definitions regardless of the subagent's `tools` config

#### Scenario: Memory tools always available to main agent
- **WHEN** the main agent loop runs (with or without subagents configured)
- **THEN** all four memory tools are present in tool definitions regardless of config

#### Scenario: Memory tools always available to subagent
- **WHEN** a subagent loop runs
- **THEN** all four memory tools are present in tool definitions regardless of the subagent's `tools` config

#### Scenario: Main agent has narrowed tool set
- **WHEN** subagents are configured and the main agent loop runs
- **THEN** the LLM only sees `comment`, `dispatch`, `skip`, `done`, `compact`, `todos`, `memory_list`, `memory_read`, `memory_write`, `memory_search` in tool definitions

#### Scenario: Full tool set when no subagents configured
- **WHEN** no subagents are configured
- **THEN** the full tool set including `todos` and all four memory tools is exposed to the LLM

#### Scenario: set_memory tool no longer present
- **WHEN** the main agent loop runs
- **THEN** `set_memory` is NOT present in tool definitions
