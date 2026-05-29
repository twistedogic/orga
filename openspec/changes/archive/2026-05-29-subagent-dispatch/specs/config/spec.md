## ADDED Requirements

### Requirement: Subagents config section
The config file SHALL support zero or more `[[subagents]]` blocks. Each block SHALL have `name` (unique string, required), `description` (string, required), and `tools` (array of tool name strings, required). Each block MAY additionally have `skills` (array of skill name strings), `model` (string, LLM model override), and `max_actions` (integer, action cap override).

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

#### Scenario: Config schema example
- **WHEN** the user adds subagents to their config
- **THEN** the following TOML structure is valid:

```toml
[[subagents]]
name = "researcher"
description = "Handles tickets that require gathering information, summarizing content, or answering factual questions."
tools = ["comment", "get_artifact", "set_memory", "return"]
skills = ["web-search"]
model = "claude-haiku-3-5"
max_actions = 15

[[subagents]]
name = "drafter"
description = "Handles tickets that require writing, drafting documents, or producing structured content."
tools = ["get_artifact", "commit_artifact", "set_memory", "return"]
max_actions = 20
```
