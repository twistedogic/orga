## Why

The `orga init` wizard sets up Trello credentials and board selection but leaves artifact store configuration entirely to manual TOML editing. Agents cannot easily onboard to a shared artifact repo without hand-crafting `[artifact]` and `[artifact.git]` config blocks.

## What Changes

- `orga init` gains an optional artifact store setup phase after the existing Trello/board steps
- The wizard asks for a local path, and if the path is absent it offers to clone from a remote URL (using the SSH agent for auth — no credentials stored)
- If no remote URL is given, the wizard initializes a local git repo at the given path
- If the path already contains a valid git repo, the wizard accepts it as-is
- Existing `[artifact]` config values are pre-filled as defaults on re-run

## Capabilities

### New Capabilities

- `init-artifact-store`: Interactive artifact store setup sub-flow within `orga init` — prompts for path, optionally clones or inits a git repo, and writes `[artifact]` + `[artifact.git]` sections to the config file

### Modified Capabilities

- `init-command`: The wizard gains an additional optional phase; existing Trello/board setup flow is unchanged

## Impact

- `src/init.rs` — new artifact setup prompts and git clone/init logic
- `src/config.rs` — `write_config_file` must include optional artifact section; `run_init` reads existing artifact config for defaults
- `git2` crate already present — used for clone and init operations
- No new dependencies required
- SSH credentials are never written to config; HTTP auth remains a manual TOML concern
