# subagent-markdown-loader Specification

## Purpose
Defines how orga discovers and parses markdown-based subagent definitions from an `agents/` directory adjacent to the config file.

## Requirements

### Requirement: Markdown agent discovery
The system SHALL scan for `*.md` files in an `agents/` directory located in the same directory as the loaded config file. If the `agents/` directory does not exist, the system SHALL silently skip discovery with no error.

#### Scenario: agents/ directory exists with markdown files
- **WHEN** the config is at `/path/to/orga.toml` and `/path/to/agents/researcher.md` exists
- **THEN** `researcher.md` is discovered and parsed as a subagent definition

#### Scenario: agents/ directory does not exist
- **WHEN** the config is at `/path/to/orga.toml` and no `/path/to/agents/` directory exists
- **THEN** no error is raised and no markdown agents are loaded

#### Scenario: agents/ directory is empty
- **WHEN** the `agents/` directory exists but contains no `*.md` files
- **THEN** no markdown agents are loaded and no error is raised

### Requirement: Markdown agent file format
Each `*.md` file SHALL consist of an optional YAML frontmatter block delimited by `---` lines followed by a markdown body. The file stem (filename without `.md`) SHALL become the subagent `name`. The document body (after the closing `---`) SHALL become the `system_prompt`.

#### Scenario: Valid markdown agent file
- **WHEN** a file `agents/researcher.md` contains valid YAML frontmatter and a body
- **THEN** the subagent name is `"researcher"` and `system_prompt` is set to the body text

#### Scenario: Frontmatter only, no body
- **WHEN** a file contains frontmatter with no content after the closing `---`
- **THEN** `system_prompt` is `None` or empty string

#### Scenario: No frontmatter, body only
- **WHEN** a file contains no `---` delimiters
- **THEN** the entire file content becomes `system_prompt` and frontmatter fields use their defaults (if any); the file is rejected as invalid since `description` is required

### Requirement: Frontmatter field mapping
The YAML frontmatter SHALL support the following fields, mapping to `SubagentConfig`:

| Frontmatter key | Type | Required | Maps to |
|---|---|---|---|
| `description` | string | YES | `description` |
| `tools` | list of strings | no | `tools` (default: `[]`) |
| `skills` | list of strings | no | `skills` (default: `[]`) |
| `max_actions` | integer | no | `max_actions` |

#### Scenario: All fields present
- **WHEN** frontmatter contains all supported fields with valid values
- **THEN** all fields are deserialized correctly into `SubagentConfig`

#### Scenario: Only required field present
- **WHEN** frontmatter contains only `description`
- **THEN** a valid `SubagentConfig` is produced with empty `tools` and `skills`, and `None` for `model` and `max_actions`

#### Scenario: description field missing
- **WHEN** frontmatter is present but does not include `description`
- **THEN** the file is skipped with a logged warning; no agent is added

### Requirement: Parse error handling
If a markdown file has malformed YAML frontmatter or fails deserialization, the system SHALL log a warning and skip that file. Other valid files in the same directory SHALL still be loaded.

#### Scenario: Malformed YAML
- **WHEN** a file's frontmatter contains invalid YAML
- **THEN** the file is skipped with a warning; remaining files are processed normally

#### Scenario: Unknown frontmatter fields
- **WHEN** frontmatter contains fields not in the schema (e.g., `author: "me"`)
- **THEN** unknown fields are ignored and the file loads successfully
