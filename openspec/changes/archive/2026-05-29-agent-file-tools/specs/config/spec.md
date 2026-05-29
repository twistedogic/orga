## ADDED Requirements

### Requirement: Workspace config section
The config file SHALL support an optional `[workspace]` section with a `path` key specifying the base directory for all ticket workspaces. If omitted, workspace tools are unavailable.

#### Scenario: Workspace configured
- **WHEN** the config contains `[workspace]\npath = "~/.orga/workspaces"`
- **THEN** `AppConfig.workspace` is `Some(WorkspaceConfig { path: "~/.orga/workspaces" })`

#### Scenario: Workspace section omitted
- **WHEN** the config does not contain a `[workspace]` section
- **THEN** `AppConfig.workspace` is `None` and the agent starts without workspace support
