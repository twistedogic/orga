## MODIFIED Requirements

### Requirement: Tool set
The agent loop SHALL expose the following tools to the LLM during a ticket cycle. Each tool SHALL correspond to an existing orga operation. In dry-run mode, mutating tools SHALL be logged but not executed; read tools SHALL execute normally. When subagents are configured, the main agent receives a narrowed tool set; subagents receive the tool set defined in their config plus `return` and `todos`. When no subagents are configured, the full tool set plus `todos` is exposed unchanged. `todos` SHALL always be available to all agents regardless of config. The four memory tools (`memory_list`, `memory_read`, `memory_write`, `memory_search`) SHALL always be available to all agents regardless of config. `move_ticket` SHALL NOT be listed in `VALID_TOOLS` until a dispatch implementation exists.

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
| `bash(command)` | subagent (if configured) | yes | shell execution |
| `todos(todos)` | all agents (always) | no | `TodoStore` scoped key |
| `memory_list()` | all agents (always) | no | `ContextRepository::list()` |
| `memory_read(path)` | all agents (always) | no | `ContextRepository::read()` |
| `memory_write(path, content, commit_msg)` | all agents (always) | yes | `ContextRepository::write()` |
| `memory_search(query)` | all agents (always) | no | `ContextRepository::search()` |

#### Scenario: todos always available to main agent
- **WHEN** the main agent loop runs (with or without subagents configured)
- **THEN** `todos` is present in the tool definitions regardless of config

#### Scenario: move_ticket rejected at validation
- **WHEN** a subagent config lists `move_ticket` in its `tools` array
- **THEN** `AppConfig::validate` returns a `ConfigError` referencing an unknown tool

#### Scenario: Single authoritative tool definition function
- **WHEN** code needs the full set of tool definitions
- **THEN** it calls `all_tool_definitions()` directly; the `tool_definitions()` alias does not exist
