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
```

All fields under `[agent]`, `[board]`, and the backend-specific section SHALL be required. The `[memory]` section is optional; if absent, the default path is used. The `member_id` field SHALL live under `[trello]`, not `[agent]`.

#### Scenario: Valid config
- **WHEN** a valid config file is loaded
- **THEN** the CLI initializes without error

#### Scenario: Missing required field
- **WHEN** a required config field is absent
- **THEN** the CLI exits with a non-zero code and prints which field is missing
