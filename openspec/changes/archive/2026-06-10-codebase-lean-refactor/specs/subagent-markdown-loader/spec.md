## MODIFIED Requirements

### Requirement: Markdown agent discovery
The system SHALL scan for `*.md` files in an `agents/` directory located in the same directory as the loaded config file. If the `agents/` directory does not exist, the system SHALL silently skip discovery with no error. The discovery and parsing logic SHALL live in `src/agent/agents.rs`, not in `src/config.rs`. `AppConfig::load` SHALL call into `agent::agents::load_markdown_agents` to populate subagents.

#### Scenario: agents/ directory exists with markdown files
- **WHEN** the config is at `/path/to/orga.toml` and `/path/to/agents/researcher.md` exists
- **THEN** `researcher.md` is discovered and parsed as a subagent definition

#### Scenario: agents/ directory does not exist
- **WHEN** the config is at `/path/to/orga.toml` and no `/path/to/agents/` directory exists
- **THEN** no error is raised and no markdown agents are loaded

#### Scenario: Logic lives in agent module
- **WHEN** `AppConfig::load` populates markdown agents
- **THEN** it delegates to `agent::agents::load_markdown_agents`; no markdown-parsing logic remains in `config.rs`
