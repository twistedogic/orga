# orga Configuration Reference

Config is loaded from `~/.orga/config.toml` by default. Override with `--config <path>` or the `ORGA_CONFIG` environment variable.

---

## `[agent]`

```toml
[agent]
name = "my-agent"
```

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `name` | string | yes | Identity used when posting comments and assigning tickets on the board. Also names the git-backed context repository. |

---

## `[board]`

```toml
[board]
backend = "trello"   # or "linear"
```

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `backend` | string | yes | Board backend. Supported: `trello`, `linear` |

---

## `[trello]`

Required when `board.backend = "trello"`.

```toml
[trello]
api_key   = "your-trello-api-key"
token     = "your-trello-token"
member_id = "your-trello-member-id"
board_id  = "your-trello-board-id"
```

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `api_key` | string | yes | Trello API key |
| `token` | string | yes | Trello OAuth token |
| `member_id` | string | yes | Your Trello member ID (used to filter assigned tickets) |
| `board_id` | string | yes | The Trello board to operate on |

---

## `[linear]`

Required when `board.backend = "linear"`.

```toml
[linear]
api_key    = "lin_api_..."
team_id    = "your-team-id"
project_id = "your-project-uuid"   # optional
```

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `api_key` | string | yes | Linear personal API key |
| `team_id` | string | yes | Linear team ID to scope issue queries |
| `project_id` | string | no | Restrict issue queries and new sub-tickets to a single Linear project. Omit to scope to the whole team. |

---

## `[llm]`

Required for `orga agent`. Not needed for read-only CLI commands.

```toml
[llm]
provider              = "anthropic"
api_key               = "sk-ant-..."
model                 = "claude-opus-4-5"
endpoint              = "https://api.anthropic.com"  # optional
poll_interval_secs    = 60                           # optional, default 60
max_actions_per_ticket = 10                          # optional, default 10
```

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `provider` | string | yes | LLM provider. Supported: `anthropic`, `openai` |
| `api_key` | string | yes | API key for the provider |
| `model` | string | yes | Model name (e.g. `claude-opus-4-5`, `gpt-4o`) |
| `endpoint` | string | no | Override the provider's default API endpoint |
| `poll_interval_secs` | integer | no | How often the agent polls for assigned tickets. Default: `60` |
| `max_actions_per_ticket` | integer | no | Maximum tool calls per ticket per cycle. Default: `10` |

---

## `[memory]`

`path` is shared between two stores: the SQLite DB at `<path>.db` (per-ticket compaction summaries and todos) and the git-backed long-term context repository at `<path>` (cross-ticket knowledge, see `memory_*` tools). Setting `path = "/var/lib/orga/m"` produces a DB at `/var/lib/orga/m.db` and a repo at `/var/lib/orga/m`.

```toml
[memory]
path                     = "~/.orga/memory"   # both DB and repo live under this prefix
defrag_file_threshold    = 20                 # optional, default 20
defrag_size_threshold_kb = 50                 # optional, default 50
```

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `path` | string | no | Base path for memory stores. SQLite DB at `<path>.db`, git repo at `<path>`. Defaults: DB `~/.orga/memory.db`, repo `~/.orga/memory`. Supports `~` expansion. |
| `defrag_file_threshold` | integer | no | Number of files in the context repo that triggers a defrag pass. Default: `20` |
| `defrag_size_threshold_kb` | integer | no | Total size of the context repo (KB) that triggers a defrag pass. Default: `50` |

---

## `[workspace]`

Required for the `bash` tool (it executes inside `<path>/<ticket-id>/`). Each ticket gets an isolated subdirectory.

```toml
[workspace]
path = "~/.orga/workspace"
```

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `path` | string | yes | Base directory for per-ticket workspaces. Each ticket gets `<path>/<ticket-id>/` as its working directory. Supports `~` expansion. |

---

## `[skills]`

```toml
[skills]
path = "~/.orga/skills"
```

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `path` | string | yes | Directory containing skill Markdown files loaded into the agent's context. Supports `~` expansion. |

---

## `[logging]`

```toml
[logging]
file  = "~/.orga/orga.log"
debug = false
```

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `file` | string | no | Path to the log file. Default: `~/.orga/orga.log`. Supports `~` expansion. |
| `debug` | bool | no | Enable debug-level log output. Default: `false` |

---

## `[metrics]`

Optional. When present, the daemon exposes Prometheus metrics on the configured address.

```toml
[metrics]
listen_addr = "127.0.0.1:9090"   # optional, default 127.0.0.1:9090
```

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `listen_addr` | string | no | `host:port` for the metrics endpoint. Default: `127.0.0.1:9090`. A bind failure is non-fatal — the daemon logs a warning and continues without metrics. |

---

## `comment_compaction_threshold`

Top-level key. Controls when the agent is allowed to suggest compacting ticket comments to reduce context size.

```toml
comment_compaction_threshold = 20
```

Default: `5`

---

## `[[subagents]]`

Defines named subagents the main agent can `dispatch` work to. Each subagent runs its own tool loop and returns a result.

Subagents can also be defined as Markdown files in an `agents/` directory next to the config file — see [Markdown subagents](#markdown-subagents) below.

```toml
[[subagents]]
name          = "researcher"
description   = "Searches the codebase and summarizes findings."
tools         = ["bash", "return"]
skills        = ["search"]
model         = "claude-haiku-4-5"     # optional, inherits [llm] model if absent
max_actions   = 20                     # optional, default from [llm]
system_prompt = "You are a research assistant..."
```

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `name` | string | yes | Unique identifier used in `dispatch { subagent: "..." }` calls |
| `description` | string | yes | Shown to the main agent when deciding which subagent to dispatch to |
| `tools` | array of string | yes | Tools this subagent may use. Valid values: `comment`, `create_sub`, `compact`, `done`, `skip`, `dispatch`, `return`, `bash`, `todos`, `memory_list`, `memory_read`, `memory_write`, `memory_search` |
| `skills` | array of string | no | Skill names (filenames without `.md`) loaded into this subagent's context from the `[skills]` directory |
| `model` | string | no | Override the LLM model for this subagent |
| `max_actions` | integer | no | Override the max tool calls per run for this subagent |
| `system_prompt` | string | no | Inline system prompt for this subagent |

---

## Markdown subagents

Subagents can be defined as `.md` files in an `agents/` directory alongside the config file. The filename (without extension) becomes the subagent name.

**`~/.orga/agents/researcher.md`**:

```markdown
---
description: Searches the codebase and summarizes findings.
tools: [bash, return]
skills: [search]
max_actions: 20
---

You are a research assistant. When given a task, use bash to investigate
and return a concise summary of your findings.
```

Frontmatter fields mirror the TOML `[[subagents]]` keys (`description`, `tools`, `skills`, `max_actions`). The Markdown body becomes `system_prompt`. `model` is not configurable via Markdown — use TOML for that.

---

## Full example (Trello)

```toml
[agent]
name = "orga-bot"

[board]
backend = "trello"

[trello]
api_key   = "abc123"
token     = "tok456"
member_id = "mem789"
board_id  = "board-xyz"

[llm]
provider               = "anthropic"
api_key                = "sk-ant-..."
model                  = "claude-opus-4-5"
poll_interval_secs     = 30
max_actions_per_ticket = 15

[memory]
path                     = "~/.orga/memory"
defrag_file_threshold    = 30
defrag_size_threshold_kb = 100

[workspace]
path = "~/.orga/workspace"

[skills]
path = "~/.orga/skills"

[logging]
file  = "~/.orga/orga.log"
debug = false

[metrics]
listen_addr = "127.0.0.1:9090"

comment_compaction_threshold = 20

[[subagents]]
name        = "bash-worker"
description = "Runs shell commands and returns results."
tools       = ["bash", "return"]
```