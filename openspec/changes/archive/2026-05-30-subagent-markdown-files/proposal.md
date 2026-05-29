## Why

Subagents are currently defined exclusively in the TOML config file, mixing agent personality (system prompt) with structural config. A markdown-based format lets users write subagent system prompts naturally as document bodies, with frontmatter for structured fields — consistent with how skills are already authored in this project and how tools like OpenCode define agents.

## What Changes

- Add support for loading subagent definitions from `*.md` files in an `agents/` directory adjacent to `orga.toml`
- File stem becomes the subagent `name`; frontmatter provides `description`, `tools`, `skills`, `max_actions`; document body becomes `system_prompt`
- `description` is required in frontmatter; all other fields are optional
- Markdown agents are additive — existing `[[subagents]]` TOML entries continue to work
- No new config keys, no global path, no override logic

## Capabilities

### New Capabilities

- `subagent-markdown-loader`: Discovers and parses `agents/*.md` files adjacent to the config file, deserializing them into `SubagentConfig` entries merged into the agent's subagent list at startup.

### Modified Capabilities

- `config`: `AppConfig::load` gains a post-load step that scans the adjacent `agents/` directory and appends parsed markdown subagents to `self.subagents`.

## Impact

- `src/config.rs` — new parsing logic, `AppConfig::load` modification
- New dependency: a YAML frontmatter parser (e.g. `gray_matter` or manual `---` split + `serde_yaml`)
- `Cargo.toml` — new crate dependency
