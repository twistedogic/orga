## Why

The native agent loop (`src/agent/`) has no awareness of domain skills — it operates with only a generic system prompt and per-column workflow instructions. Adding skills support lets operators teach the agent how to behave on specific types of work, and lets humans hint which skills apply to a ticket by adding `skill:` prefixed labels.

## What Changes

- New `[skills]` config section pointing to a folder of agentskills.io-compliant skill directories
- At ticket-load time, scan the skills folder, list all skills (name + description) in the system prompt, and inject the full body of any matched skills into an "Active Skills" section
- Skill matching: `orga-match-always`, `orga-match-column`, `orga-match-label` metadata keys in each skill's frontmatter, plus `skill:<name>` ticket labels as a human override
- Multiple matched skills are concatenated; missing skills referenced via `skill:` labels produce a logged warning

## Capabilities

### New Capabilities

- `agent-skills`: Discovery, matching, and injection of agentskills.io-compliant skills into the agent's system prompt at ticket-load time

### Modified Capabilities

- `config`: New `[skills]` section with a `path` field pointing to the skills folder
- `agent-loop`: System prompt construction extended to include available skills listing and active skills injection

## Impact

- `src/config.rs` — new `SkillsConfig` struct, added to `AppConfig`
- `src/agent/context.rs` — `build_system_prompt` extended to accept and render skill content
- New `src/agent/skills.rs` — skill scanning, frontmatter parsing, and matching logic
- No new dependencies required (frontmatter parsing via simple string splitting; no external crate needed)
- No breaking changes to CLI surface or existing config files (skills section is optional)
