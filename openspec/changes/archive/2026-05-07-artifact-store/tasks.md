## 1. Dependencies & Config

- [x] 1.1 Add `git2` crate to `Cargo.toml`
- [x] 1.2 Add `ArtifactConfig` and `ArtifactGitConfig` structs to `src/config.rs`
- [x] 1.3 Add `artifact: Option<ArtifactConfig>` field to `AppConfig`
- [x] 1.4 Write config tests: valid artifact section, missing artifact section, missing `path` field

## 2. Models

- [x] 2.1 Add `ArtifactMeta` struct to `src/models.rs` (`ticket_id`, `agent_name`, `name`, `committed_at`)
- [x] 2.2 Add `Artifact` struct to `src/models.rs` (flattens `ArtifactMeta` + `content: String`)

## 3. ArtifactStore Trait

- [x] 3.1 Create `src/artifact/mod.rs` with `ArtifactStore` trait (`commit`, `get`, `list`)
- [x] 3.2 Implement `build_artifact_store` factory in `src/artifact/mod.rs`
- [x] 3.3 Export `artifact` module from `src/lib.rs`

## 4. Git Backend

- [x] 4.1 Create `src/artifact/git.rs` with `GitArtifactStore` struct holding repo path, agent name, optional remote, and branch
- [x] 4.2 Implement `commit`: write file to `artifacts/<ticket-id>/<agent-name>/<name>`, stage, git commit
- [x] 4.3 Implement fetch → rebase → push sequence in `commit` when remote is configured
- [x] 4.4 Handle rebase conflict: abort rebase, clean up, return `OrgaError::BackendError`
- [x] 4.5 Implement `list`: walk `artifacts/<ticket-id>/` directory, collect `ArtifactMeta` for all agents
- [x] 4.6 Implement `get`: read `artifacts/<ticket-id>/<agent-name>/<name>`, return `Option<Artifact>`
- [x] 4.7 Write unit tests for `commit` (inline content, file content, overwrite existing)
- [x] 4.8 Write unit tests for `list` (empty, single agent, multiple agents)
- [x] 4.9 Write unit tests for `get` (found, not found)

## 5. CLI Subcommand

- [x] 5.1 Add `Commands::Artifact(ArtifactCommands)` to `src/main.rs`
- [x] 5.2 Add `ArtifactCommands::Commit` with `ticket_id`, `name`, optional `content`, and `--file` flag
- [x] 5.3 Add validation: error if neither content nor `--file` provided; error if both provided
- [x] 5.4 Add `ArtifactCommands::List` with `ticket_id`
- [x] 5.5 Add `ArtifactCommands::Get` with `ticket_id` and `name`
- [x] 5.6 Implement human-readable output for all three commands
- [x] 5.7 Implement `--json` output for all three commands
- [x] 5.8 Wire `build_artifact_store` call in the `Commands::Artifact` dispatch arm

## 6. Integration Tests

- [x] 6.1 Add integration test: `artifact commit` with inline content round-trips via `artifact get`
- [x] 6.2 Add integration test: `artifact commit` with `--file` reads file and commits
- [x] 6.3 Add integration test: `artifact list` shows artifacts from multiple agents
- [x] 6.4 Add integration test: `artifact get` on missing artifact exits non-zero
