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
trello_member_id = "abc123"

[board]
id = "board-xyz"
backend = "trello"

[trello]
api_key = "..."
token = "..."

[memory]
path = "~/.orga/memory.db"
```

All fields under `[agent]`, `[board]`, and the backend-specific section SHALL be required. The `[memory]` section is optional; if absent, the default path is used.

#### Scenario: Valid config
- **WHEN** a valid config file is loaded
- **THEN** the CLI initializes without error

#### Scenario: Missing required field
- **WHEN** a required config field is absent
- **THEN** the CLI exits with a non-zero code and prints which field is missing

### Requirement: Config validation
The CLI SHALL validate the config at startup before executing any command. Unknown backend values SHALL produce an error.

#### Scenario: Unknown backend
- **WHEN** `board.backend` is set to an unrecognized value
- **THEN** the CLI exits with a non-zero code listing supported backends

#### Scenario: Missing config file
- **WHEN** the config file does not exist at the resolved path
- **THEN** the CLI exits with a non-zero code and prints the expected config path

