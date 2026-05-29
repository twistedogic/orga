# config Specification

## Purpose
TBD - created by archiving change agent-board-cli. Update Purpose after archive.
## Requirements
### Requirement: TOML config file loading
The CLI SHALL load configuration from a TOML file. The default path SHALL be `~/.orga/config.toml`, overridable via the `--config <path>` global flag or `ORGA_CONFIG` environment variable.

#### Scenario: Config loaded from default path
- **WHEN** the CLI is invoked without `--config`
- **THEN** config is loaded from `~/.orga/config.toml`

#### Scenario: Config loaded from explicit path
- **WHEN** `--config /path/to/config.toml` is passed
- **THEN** config is loaded from the specified path

#### Scenario: Config loaded from environment variable
- **WHEN** `ORGA_CONFIG=/path/to/config.toml` is set
- **THEN** config is loaded from that path (overridden by `--config` if both are set)

### Requirement: Config schema
The config file SHALL support the following structure:

```toml
[agent]
name = "agent-1"

[board]
backend = "trello"

[trello]
api_key = "..."
token = "..."
member_id = "abc123"
board_id = "board-xyz"

[memory]
path = "~/.orga/memory.db"

[artifact]
backend = "git"

[artifact.git]
path = "~/.orga/artifacts"
remote = "origin"
branch = "main"

[logging]
file = "~/.orga/orga.log"
debug = false

[[workflow]]
column = "To Do"
prompt = "Enter explore mode..."

[[workflow]]
column = "In Progress"
prompt_file = "~/.orga/prompts/in-progress.md"

[llm]
provider = "anthropic"              # required: "anthropic" | "openai"
api_key = "sk-ant-..."              # required
model = "claude-opus-4-5"           # required
endpoint = "https://..."            # optional: overrides provider default base URL
poll_interval_secs = 60             # optional: default 60
max_actions_per_ticket = 10         # optional: default 10
max_artifact_inline_bytes = 8192    # optional: default 8192

[skills]
path = "~/.orga/skills"             # optional: path to skills folder
```

All fields under `[agent]`, `[board]`, and the backend-specific section SHALL be required. The `[memory]` section is optional; if absent, the default path is used. The `member_id` field SHALL live under `[trello]`, not `[agent]`. The `[artifact]` section is optional; if absent, artifact commands fail with a clear config error. Within `[artifact.git]`, `path` is required; `remote` and `branch` are optional (defaulting to no remote and `"main"` respectively). The `[logging]` section is optional; if absent, `file` defaults to `~/.orga/orga.log` and `debug` defaults to `false`. The `[[workflow]]` section is optional; if absent, no workflow prompts are injected. Each `[[workflow]]` entry requires `column` and exactly one of `prompt` or `prompt_file`. The `[llm]` section is optional; if absent, `orga agent` fails with a clear config error; all other commands are unaffected. The `[skills]` section is optional; if absent, no skills are loaded and the system prompt contains no skills sections.

#### Scenario: Valid config
- **WHEN** a valid config file is loaded
- **THEN** the CLI initializes without error

#### Scenario: Missing required field
- **WHEN** a required config field is absent
- **THEN** the CLI exits with a non-zero code and prints which field is missing

#### Scenario: Config with skills section
- **WHEN** a config file includes `[skills]` with a valid `path`
- **THEN** the skills folder is used for skill discovery during agent runs

#### Scenario: Config without skills section
- **WHEN** no `[skills]` section is present
- **THEN** the agent runs without skills; no skills sections appear in system prompts

#### Scenario: Config with artifact section
- **WHEN** a config file includes a valid `[artifact]` and `[artifact.git]` section
- **THEN** `build_artifact_store` succeeds and returns a `GitArtifactStore`

#### Scenario: Config without artifact section
- **WHEN** the config file does not include `[artifact]`
- **THEN** artifact commands fail with a config error; all other commands are unaffected

#### Scenario: Config with workflow section
- **WHEN** a config file includes valid `[[workflow]]` entries
- **THEN** the config loads successfully and workflow prompts are available for matching columns

#### Scenario: Config without workflow section
- **WHEN** the config file does not include `[[workflow]]`
- **THEN** the config loads successfully; no workflow prompts are injected

#### Scenario: Config with logging section
- **WHEN** a config file includes `[logging]` with a `file` path and `debug = true`
- **THEN** the logger writes to the specified path and debug entries are emitted

#### Scenario: Config without logging section
- **WHEN** the config file does not include `[logging]`
- **THEN** the logger uses `~/.orga/orga.log` as the default path and debug is disabled

#### Scenario: Valid config with llm section
- **WHEN** a valid config file includes a `[llm]` section with `provider`, `api_key`, and `model`
- **THEN** the CLI initializes without error and `orga agent` can run

#### Scenario: Missing llm section does not affect other commands
- **WHEN** the config file does not include `[llm]`
- **THEN** all commands except `orga agent` work normally; `orga agent` exits with a config error

#### Scenario: llm section with endpoint override
- **WHEN** `[llm]` includes `endpoint = "https://proxy.example.com/v1"`
- **THEN** the LLM client uses that endpoint instead of the provider default

#### Scenario: llm section missing required fields
- **WHEN** `[llm]` is present but missing `api_key` or `model`
- **THEN** the CLI exits with a config error naming the missing field

### Requirement: Config validation
The CLI SHALL validate the config at startup before executing any command. Unknown backend values SHALL produce an error.

#### Scenario: Unknown backend
- **WHEN** `board.backend` is set to an unrecognized value
- **THEN** the CLI exits with a non-zero code listing supported backends

#### Scenario: Missing config file
- **WHEN** the config file does not exist at the resolved path
- **THEN** the CLI exits with a non-zero code and prints the expected config path

### Requirement: Subagents config section
The config file SHALL support zero or more `[[subagents]]` blocks. Each block SHALL have `name` (unique string, required), `description` (string, required), and `tools` (array of tool name strings, required). Each block MAY additionally have `skills` (array of skill name strings), `model` (string, LLM model override), and `max_actions` (integer, action cap override). Config validation SHALL reject duplicate subagent names and unknown tool names.

#### Scenario: Subagents section parsed from config
- **WHEN** the config file contains one or more `[[subagents]]` blocks
- **THEN** they are deserialized into a list of `SubagentConfig` and available to the agent loop

#### Scenario: Config with subagent model override
- **WHEN** a `[[subagents]]` block contains `model = "claude-opus-4-5"`
- **THEN** the subagent loop uses that model instead of the global `[llm].model`

#### Scenario: Validation rejects duplicate subagent names
- **WHEN** two `[[subagents]]` blocks have the same `name`
- **THEN** config validation fails with a descriptive error at startup

#### Scenario: Validation rejects unknown tool names in subagent
- **WHEN** a `[[subagents]]` block lists a tool name that does not exist (e.g., `tools = ["fly"]`)
- **THEN** config validation fails with a descriptive error at startup

### Requirement: Workspace config section
The config file SHALL support an optional `[workspace]` section with a `path` key specifying the base directory for all ticket workspaces. If omitted, workspace tools are unavailable.

#### Scenario: Workspace configured
- **WHEN** the config contains `[workspace]\npath = "~/.orga/workspaces"`
- **THEN** `AppConfig.workspace` is `Some(WorkspaceConfig { path: "~/.orga/workspaces" })`

#### Scenario: Workspace section omitted
- **WHEN** the config does not contain a `[workspace]` section
- **THEN** `AppConfig.workspace` is `None` and the agent starts without workspace support
