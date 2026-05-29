## Why

The artifact store (git-backed file storage per ticket) is not used in practice. The workspace module (`read_file`, `write_file`, `list_files`) already covers persistent file I/O per ticket. Keeping the artifact store adds maintenance burden, config surface area, and dead code paths with no real benefit.

## What Changes

- Remove `src/artifact/` module entirely (`ArtifactStore` trait + `GitArtifactStore` implementation)
- Remove `commit_artifact` and `get_artifact` agent tools
- Remove artifact list injection from agent context/prompt
- Remove `[artifact]` and `[artifact.git]` config sections; presence of either in a config file SHALL produce a startup error directing users to workspace
- Remove `max_artifact_inline_bytes` field from `LlmConfig`
- Remove `orga artifact` CLI subcommand (commit/get/list)
- Remove artifact section from `orga init` config writer
- Remove `pub mod artifact` from `src/lib.rs`

## Capabilities

### New Capabilities

*(none)*

### Modified Capabilities

- `config`: `[artifact]` and `[artifact.git]` sections are now invalid; validation SHALL reject configs that include them with a clear migration message
- `agent-tools`: `commit_artifact` and `get_artifact` are removed from the tool set and from valid tool names in subagent config validation
- `cli-commands`: `orga artifact` subcommand is removed
- `llm-client`: `max_artifact_inline_bytes` config field is removed

## Impact

- `src/artifact/` — deleted
- `src/lib.rs` — remove `pub mod artifact`
- `src/config.rs` — remove `ArtifactConfig`, `ArtifactGitConfig`, `artifact` field, `max_artifact_inline_bytes`; add validation error for `[artifact]` presence; remove `commit_artifact`/`get_artifact` from `VALID_TOOLS`
- `src/main.rs` — remove `artifact` subcommand, remove `build_artifact_store` call
- `src/agent/tools.rs` — remove `commit_artifact`, `get_artifact` dispatch + definitions, remove `artifact_store` from `ToolContext`
- `src/agent/context.rs` — remove artifact list injection, remove `artifact_store` param
- `src/agent/mod.rs` — remove all `build_artifact_store` calls
- `src/init.rs` — remove artifact section from config writer
- `Cargo.toml` — remove `git2` dependency (only used by artifact git backend)
