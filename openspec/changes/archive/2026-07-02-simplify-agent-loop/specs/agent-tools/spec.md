## MODIFIED Requirements

### Requirement: Tool set
The agent loop SHALL expose the following tools to the LLM during a ticket cycle. Each tool SHALL correspond to an existing orga operation. In dry-run mode, mutating tools SHALL be logged but not executed; read tools SHALL execute normally. When subagents are configured, the main agent receives a narrowed tool set; subagents receive the tool set defined in their config plus `return` and `todos`. When no subagents are configured, the full tool set plus `todos` is exposed unchanged. `todos` SHALL always be available to all agents regardless of config. The four memory tools (`memory_list`, `memory_read`, `memory_write`, `memory_search`) SHALL always be available to all agents regardless of config. `move_ticket` SHALL NOT be listed in `VALID_TOOLS` until a dispatch implementation exists.

The set of tools listed in the agent's system prompt under "Available tools: …" SHALL exactly equal the set of tools the code path actually exposes to the LLM. Both MUST be derived from a single `pub const MAIN_TOOLS: &[&str]` declared in `src/agent/tools.rs`. Adding, removing, or renaming a tool SHALL be a single edit to that constant; the prompt text MUST NOT list tools that the constant does not include, and MUST NOT omit tools that the constant does include.

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

#### Scenario: comment tool posts to board
- **WHEN** the LLM calls `comment(text: "update: work started")`
- **THEN** `board.comment(ticket_id, text)` is called and the result is returned to the LLM

#### Scenario: done tool returns ticket
- **WHEN** the LLM calls `done(comment: "work complete")`
- **THEN** `board.return_ticket(ticket_id, Some("work complete"))` is called and the cycle ends

#### Scenario: done tool without comment
- **WHEN** the LLM calls `done()` with no comment
- **THEN** `board.return_ticket(ticket_id, None)` is called

#### Scenario: skip tool ends cycle silently
- **WHEN** the LLM calls `skip()`
- **THEN** no board mutation occurs and the cycle ends

#### Scenario: move_ticket rejected at validation
- **WHEN** a subagent config lists `move_ticket` in its `tools` array
- **THEN** `AppConfig::validate` returns a `ConfigError` referencing an unknown tool

#### Scenario: Single authoritative tool definition function
- **WHEN** code needs the full set of tool definitions
- **THEN** it calls `all_tool_definitions()` directly; the `tool_definitions()` alias does not exist

#### Scenario: System prompt tool list matches exposed tools
- **WHEN** the system prompt is rendered for any agent (main, dispatcher, subagent)
- **THEN** every tool named under "Available tools: …" is also present in the tool definitions sent to the LLM in that turn, and no exposed tool is omitted from the prompt text

#### Scenario: Single source of truth for main-agent tool set
- **WHEN** the set of main-agent tools needs to change
- **THEN** the only edit required is to `pub const MAIN_TOOLS: &[&str]` in `src/agent/tools.rs`; the prompt text and the runtime tool definitions both reflect the change without further code edits
