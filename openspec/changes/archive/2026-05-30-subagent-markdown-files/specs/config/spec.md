# config Specification (delta)

## MODIFIED Requirements

### Requirement: Subagents config section
The config file SHALL support zero or more `[[subagents]]` blocks. Each block SHALL have `name` (unique string, required), `description` (string, required), and `tools` (array of tool name strings, required). Each block MAY additionally have `skills` (array of skill name strings), `model` (string, LLM model override), and `max_actions` (integer, action cap override). Config validation SHALL reject duplicate subagent names and unknown tool names.

In addition, the system SHALL load subagent definitions from `*.md` files in an `agents/` directory adjacent to the config file (see `subagent-markdown-loader` spec). Markdown agents are appended to the TOML-defined subagent list after parsing. The combined list (TOML + markdown) is subject to all existing validation rules.

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

#### Scenario: Markdown agents loaded alongside TOML subagents
- **WHEN** the config has a TOML `[[subagents]]` entry and an `agents/` directory with a valid `.md` file
- **THEN** both are loaded and merged into a single subagent list

#### Scenario: Markdown-only subagents (no TOML subagents)
- **WHEN** the config has no `[[subagents]]` blocks but has a valid `agents/*.md` file
- **THEN** the markdown agent is loaded and available to the agent loop
