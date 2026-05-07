## Why

Agents produce deliverables — reports, analysis, generated files — but have no structured way to store and share them per-ticket. The memory store is private scratchpad; there is no mechanism for an agent to commit outputs that persist, are versioned, and are accessible to other agents or humans reviewing a ticket's work.

## What Changes

- New `ArtifactStore` trait — backend-agnostic abstraction for committing and retrieving artifacts
- New `GitArtifactStore` backend — pilot implementation storing artifacts in a dedicated git repo, namespaced by ticket and agent, with auto-rebase and push on every commit
- New `orga artifact` CLI subcommand — `commit`, `list`, and `get` operations
- New `[artifact]` config section — backend selection, repo path, and optional remote

## Capabilities

### New Capabilities

- `artifact-store`: The `ArtifactStore` trait and factory function, mirroring the `Board` trait pattern
- `artifact-git-backend`: Git-based implementation storing artifacts at `artifacts/<ticket-id>/<agent-name>/<name>` in a dedicated repo, committing and pushing on every write
- `artifact-cli`: The `orga artifact` subcommand exposing `commit`, `list`, and `get`

### Modified Capabilities

- `config`: New optional `[artifact]` and `[artifact.git]` config sections

## Impact

- New module: `src/artifact/mod.rs`, `src/artifact/git.rs`
- `src/config.rs` — add `ArtifactConfig` and `ArtifactGitConfig` structs
- `src/main.rs` — add `Commands::Artifact` and `ArtifactCommands` subcommands
- `src/lib.rs` — expose `artifact` module
- New dependency: `git2` crate for git operations
