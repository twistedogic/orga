## Why

First-time setup requires manually writing a TOML config file and locating non-obvious values like Trello board IDs and member IDs. An `init` command eliminates that friction by guiding the user through setup with an interactive TUI form, fetching values automatically where possible.

## What Changes

- New `orga init` top-level command that launches an interactive setup wizard
- The wizard collects agent name, Trello credentials, then auto-fetches the member ID and presents a board picker populated from the Trello API
- If a config file already exists, current values are shown as defaults in each prompt
- Writes a valid `~/.orga/config.toml` (or the path from `--config` / `ORGA_CONFIG`) on completion
- `init` runs before config is loaded — it must not require a valid config to already exist

## Capabilities

### New Capabilities

- `init-command`: Interactive TUI wizard for first-time config setup using `inquire`; fetches Trello member ID and board list from the API; writes the resulting TOML config file

### Modified Capabilities

- `config`: The config spec requires a `[trello] member_id` field, but the current schema has it under `[agent]`. The `init` command will write `member_id` under `[trello]` to match the current code. The spec needs a correction to reflect the actual schema.
- `cli-commands`: `init` is a new top-level command (not under `ticket`, `checklist`, or `memory`); the spec needs to document it

## Impact

- New dependency: `inquire` crate
- New module: `src/init.rs`
- `src/main.rs`: adds `Commands::Init` variant; `init` path skips `AppConfig::load()`
- `src/config.rs`: adds `try_load()` returning `Option<AppConfig>` for pre-populating defaults
- No changes to existing commands, `Board` trait, or `MemoryStore`
