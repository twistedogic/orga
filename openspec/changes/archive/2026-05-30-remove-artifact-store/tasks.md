## 1. Delete artifact module

- [ ] 1.1 Delete `src/artifact/` directory (mod.rs + git.rs)
- [ ] 1.2 Remove `pub mod artifact` from `src/lib.rs`

## 2. Update config

- [ ] 2.1 Remove `ArtifactConfig` and `ArtifactGitConfig` structs from `src/config.rs`
- [ ] 2.2 Remove `artifact` field from `AppConfig`
- [ ] 2.3 Remove `max_artifact_inline_bytes` field from `LlmConfig` and its accessor method
- [ ] 2.4 Add validation in `AppConfig::validate` to reject configs containing `[artifact]` with a clear error message
- [ ] 2.5 Remove `commit_artifact` and `get_artifact` from `VALID_TOOLS` in `validate`

## 3. Update agent tools

- [ ] 3.1 Remove `artifact_store` field from `ToolContext` in `src/agent/tools.rs`
- [ ] 3.2 Remove `dispatch_commit_artifact` and `dispatch_get_artifact` functions
- [ ] 3.3 Remove `commit_artifact` and `get_artifact` match arms from `dispatch`
- [ ] 3.4 Remove `commit_artifact` and `get_artifact` from `tool_definitions()`

## 4. Update agent context

- [ ] 4.1 Remove `artifact_store` parameter from `build_context` and `build_subagent_context` in `src/agent/context.rs`
- [ ] 4.2 Remove artifact list injection from `build_user_message`
- [ ] 4.3 Remove `use crate::artifact::ArtifactStore` import

## 5. Update agent loop

- [ ] 5.1 Remove all `build_artifact_store` calls from `src/agent/mod.rs`
- [ ] 5.2 Remove `artifact_store` from `ToolContext` construction sites
- [ ] 5.3 Remove `artifact_store_opt` from `build_context` call sites
- [ ] 5.4 Remove `use crate::artifact::build_artifact_store` import

## 6. Update CLI

- [ ] 6.1 Remove `artifact` subcommand and all its handlers from `src/main.rs`
- [ ] 6.2 Remove `use orga::artifact::build_artifact_store` import from `src/main.rs`

## 7. Update init

- [ ] 7.1 Remove `run_artifact_setup` function from `src/init.rs`
- [ ] 7.2 Remove `artifact` parameter from `write_config_file` and `write_linear_config_file`
- [ ] 7.3 Remove artifact section writing logic from those functions
- [ ] 7.4 Remove `clone_with_ssh_key_or_agent` function (only used for artifact repo cloning)
- [ ] 7.5 Remove `use git2::Repository` and all `git2` usage from `src/init.rs`

## 8. Remove git2 dependency

- [ ] 8.1 Remove `git2` from `Cargo.toml`

## 9. Fix tests

- [ ] 9.1 Remove artifact-related tests from `src/config.rs`
- [ ] 9.2 Remove artifact-related tests from `src/agent/context.rs`
- [ ] 9.3 Remove artifact-related tests from `src/agent/tools.rs`
- [ ] 9.4 Remove artifact-related tests from `tests/integration_test.rs`
- [ ] 9.5 Remove artifact-related tests from `src/init.rs`
- [ ] 9.6 Verify `cargo test` passes with zero failures
