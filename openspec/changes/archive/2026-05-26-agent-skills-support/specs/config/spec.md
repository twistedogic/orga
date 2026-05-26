## MODIFIED Requirements

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
