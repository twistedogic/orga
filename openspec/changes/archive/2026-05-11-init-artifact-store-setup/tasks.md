## 1. Config layer

- [x] 1.1 Extend `write_config_file` in `src/config.rs` to accept `Option<ArtifactGitConfig>` and append `[artifact]` + `[artifact.git]` TOML blocks when `Some`
- [x] 1.2 Update `write_config_file` unit tests to cover: no artifact section, local-only artifact section, artifact section with remote and branch

## 2. Init wizard — artifact sub-flow

- [x] 2.1 Add `run_artifact_setup` function in `src/init.rs` that returns `Option<ArtifactGitConfig>` (None = skipped)
- [x] 2.2 Implement the opt-in confirmation prompt ("Configure artifact store?")
- [x] 2.3 Implement the local path prompt with default `~/.orga/artifacts`, pre-filled from existing config on re-run
- [x] 2.4 Implement path-state detection: valid git repo → accept as-is; exists but not a repo → return error; missing → proceed to clone/init branch
- [x] 2.5 Implement the remote URL prompt (blank = local-only)
- [x] 2.6 Implement `git2::Repository::init` for the local-only path
- [x] 2.7 Implement `git2::Repository::clone` with SSH agent callbacks for the remote URL path, including branch and remote name prompts (defaults: `main`, `origin`)
- [x] 2.8 Wire `run_artifact_setup` into `run_init` after the board selection step, passing its result into `write_config_file`

## 3. Tests

- [x] 3.1 Test `run_artifact_setup` skipped path: `write_config_file` called with `None`, no artifact sections in written config
- [x] 3.2 Test local-init path: missing dir is created, `Repository::open` succeeds, config written with `path` only
- [x] 3.3 Test existing-repo path: valid repo accepted, config written with `path` only (no remote)
- [x] 3.4 Test error path: existing non-repo dir returns `OrgaError::ConfigError`
