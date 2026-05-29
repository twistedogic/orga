## Context

Subagents are defined via `[[subagents]]` entries in `orga.toml`. Each entry has structured fields (`name`, `description`, `tools`, `skills`, `max_actions`) and an optional `system_prompt` string. Writing multi-line system prompts in TOML is awkward — triple-quoted strings lose readability and can't be edited with markdown tooling. Skills in this project are already authored as markdown files with frontmatter, establishing a precedent.

The config currently depends on `toml` and `serde` only — no YAML parser exists in the tree.

## Goals / Non-Goals

**Goals:**
- Load `*.md` files from an `agents/` directory adjacent to `orga.toml` as additional subagents
- Parse YAML frontmatter for structured fields; document body becomes system prompt
- `description` required; all other fields optional
- Additive — TOML subagents continue to work unchanged
- Single new crate dependency

**Non-Goals:**
- Global agent directory (`~/.orga/agents/`)
- Config key pointing to a custom agents directory
- Override/merge semantics between TOML and markdown agents
- Permission model (keep `tools` as `Vec<String>`)
- Validation beyond `description` presence

## Decisions

**Frontmatter parsing: manual split + `serde_yaml`**

The file format is: optional `---\n<yaml>\n---\n<body>`. Splitting on the `---` delimiter is trivial (~10 lines). `serde_yaml` deserializes the YAML block into a struct. This avoids pulling in `gray_matter` (extra dependency, less control) while staying simple.

`serde_yaml` is widely used in the Rust ecosystem and has no surprising transitive deps.

**Discovery: sibling `agents/` directory, not configurable**

The `agents/` dir path is derived from `config_path.parent() / "agents"`. No new config key. If the directory doesn't exist, silently skip — no error. This keeps the surface area minimal and avoids touching the config schema.

**Merge point: `AppConfig::load` appends after TOML parse**

After `toml::from_str`, call a `load_markdown_agents(config_dir)` function that returns `Vec<SubagentConfig>`. Append to `self.subagents`. This is the least invasive integration point and keeps markdown loading isolated from TOML parsing.

**Duplicate names**: not handled — first caller wins (TOML first). No dedup for now.

## Risks / Trade-offs

- **`serde_yaml` adds a dependency** → it's mature and widely used; acceptable tradeoff for clean YAML parsing
- **No validation of duplicate names** → acceptable for v1; agents list is short and operator-controlled
- **Silent skip if `agents/` missing** → could confuse users who misconfigure the path, but matches how skills scanning works today

## Migration Plan

No migration needed — additive change. Existing `orga.toml` files with `[[subagents]]` continue to work. New `agents/` directory is opt-in.
