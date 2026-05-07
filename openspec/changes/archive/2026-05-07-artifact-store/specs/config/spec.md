## MODIFIED Requirements

### Requirement: Config schema
The config file SHALL support the following structure:

```toml
[agent]
name = "agent-1"

[board]
id = "board-xyz"
backend = "trello"

[trello]
api_key = "..."
token = "..."
member_id = "abc123"

[memory]
path = "~/.orga/memory.db"

[artifact]
backend = "git"

[artifact.git]
path = "~/.orga/artifacts"
remote = "origin"
branch = "main"
```

All fields under `[agent]`, `[board]`, and the backend-specific section SHALL be required. The `[memory]` section is optional; if absent, the default path is used. The `member_id` field SHALL live under `[trello]`, not `[agent]`. The `[artifact]` section is optional; if absent, artifact commands fail with a clear config error. Within `[artifact.git]`, `path` is required; `remote` and `branch` are optional (defaulting to no remote and `"main"` respectively).

#### Scenario: Valid config
- **WHEN** a valid config file is loaded
- **THEN** the CLI initializes without error

#### Scenario: Missing required field
- **WHEN** a required config field is absent
- **THEN** the CLI exits with a non-zero code and prints which field is missing

#### Scenario: Config with artifact section
- **WHEN** a config file includes a valid `[artifact]` and `[artifact.git]` section
- **THEN** `build_artifact_store` succeeds and returns a `GitArtifactStore`

#### Scenario: Config without artifact section
- **WHEN** the config file does not include `[artifact]`
- **THEN** artifact commands fail with a config error; all other commands are unaffected
