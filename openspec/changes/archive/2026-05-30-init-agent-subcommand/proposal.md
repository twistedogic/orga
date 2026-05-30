## Why

`orga init` currently only sets up board credentials (Trello/Linear), leaving `[llm]` — required for `orga agent` — with no setup path. Users must hand-edit TOML to enable agent mode, which is error-prone and undiscoverable.

## What Changes

- **BREAKING**: `orga init` is replaced by two subcommands: `orga init board` and `orga init agent`
- `orga init board` — existing board/credentials wizard, behavior unchanged
- `orga init agent` — new interactive wizard that sets up `[llm]` (and optionally `[memory]`, `[workspace]`, `[skills]`)
- `AppConfig` and all sub-structs gain `Serialize` + `skip_serializing_if` attributes, enabling round-trip TOML serialization
- Config write logic is unified: a single `AppConfig::save(path)` replaces the manual format-string approach in `run_trello_init` and `run_linear_init`

## Capabilities

### New Capabilities
- `init-agent-command`: Interactive wizard for `orga init agent` — prompts for LLM provider, API key, model, and optional memory/workspace/skills paths; merges into existing config

### Modified Capabilities
- `init-command`: `orga init` is now a subcommand group; `orga init board` replaces the top-level `orga init`; existing behavior is preserved under the new path
- `config`: `AppConfig` gains `Serialize` support; all `Option<_>` fields get `skip_serializing_if = "Option::is_none"`, all `Vec<_>` fields get `skip_serializing_if = "Vec::is_empty"`; a `save(path)` method is added

## Impact

- `src/main.rs` — `Commands::Init` becomes `Commands::Init(InitCommands)`; dispatch updated
- `src/init.rs` — `run_init` renamed to `run_board_init`; new `run_agent_init` added; both use `AppConfig::save`
- `src/config.rs` — `Serialize` added to all structs; `skip_serializing_if` attributes added; `AppConfig::save` method added
- All existing tests for `AppConfig::load` are unaffected; new tests cover round-trip serialization and `save`
- No changes to board backends, agent loop, or any other commands
