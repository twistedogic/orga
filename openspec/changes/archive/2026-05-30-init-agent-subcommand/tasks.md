## 1. Config Serialization

- [x] 1.1 Add `Serialize` to all structs in `src/config.rs` (`AgentConfig`, `BoardConfig`, `TrelloConfig`, `LinearConfig`, `MemoryConfig`, `WorkflowEntry`, `WorkspaceConfig`, `SkillsConfig`, `SubagentConfig`, `LoggingConfig`, `LlmConfig`, `AppConfig`)
- [x] 1.2 Add `#[serde(skip_serializing_if = "Option::is_none")]` to all `Option<_>` fields in `AppConfig` and sub-structs
- [x] 1.3 Add `#[serde(skip_serializing_if = "Vec::is_empty")]` to `workflow` and `subagents` fields in `AppConfig`
- [x] 1.4 Implement `AppConfig::save(path: &Path) -> Result<(), OrgaError>` using `toml::to_string` + `fs::write`
- [x] 1.5 Add unit tests: round-trip serialize/deserialize, `None` fields omitted, empty `Vec` fields omitted, `save` creates parent dirs

## 2. Refactor Board Init to Use AppConfig::save

- [x] 2.1 Refactor `run_trello_init` to build an `AppConfig` struct and call `config.save(path)` instead of `write_config_file`
- [x] 2.2 Refactor `run_linear_init` to build an `AppConfig` struct and call `config.save(path)` instead of `write_linear_config_file`
- [x] 2.3 Delete `write_config_file` and `write_linear_config_file` helper functions
- [x] 2.4 Verify existing board init tests still pass

## 3. CLI Subcommand Split

- [x] 3.1 Add `InitCommands` enum to `src/main.rs` with `Board` and `Agent` variants
- [x] 3.2 Change `Commands::Init` from a unit variant to `Init(InitCommands)`
- [x] 3.3 Update dispatch in `main.rs`: `Commands::Init(InitCommands::Board)` calls `run_board_init`, `Commands::Init(InitCommands::Agent)` calls `run_agent_init`
- [x] 3.4 Rename `run_init` to `run_board_init` in `src/init.rs` and update the `pub use` in `src/lib.rs` / import in `main.rs`

## 4. Agent Init Wizard

- [x] 4.1 Implement `run_agent_init(config_path: &Path) -> Result<(), OrgaError>` in `src/init.rs`
- [x] 4.2 Load existing config (if any) to pre-populate defaults
- [x] 4.3 Prompt for provider via `Select` (`anthropic`, `openai`) with existing value pre-selected
- [x] 4.4 Prompt for API key (masked password, blank = keep existing)
- [x] 4.5 Prompt for model with provider-based default (`claude-opus-4-5` / `gpt-4o`) and existing value override
- [x] 4.6 Prompt for optional memory path, workspace path, skills path (blank = skip / keep existing)
- [x] 4.7 Merge new `[llm]`, `[memory]`, `[workspace]`, `[skills]` values into loaded config (preserving all other sections)
- [x] 4.8 Call `config.save(path)` and self-validate via `AppConfig::load`
- [x] 4.9 Print confirmation: `Config written to <path>`

## 5. Spec Verification

- [x] 5.1 Verify `orga init` alone prints subcommand help and exits non-zero
- [x] 5.2 Verify `orga init board` runs the existing wizard end-to-end
- [x] 5.3 Verify `orga init agent` runs the new wizard and produces a loadable config
- [x] 5.4 Verify `orga init agent` on an existing config with `[trello]` preserves the trello section
