## Context

The artifact store is a git-backed file storage system allowing agents to commit and retrieve named files per ticket. It has a trait (`ArtifactStore`), one implementation (`GitArtifactStore` in `src/artifact/git.rs`), two agent tools (`commit_artifact`, `get_artifact`), a CLI subcommand (`orga artifact`), and a prompt injection that lists existing artifacts in the agent context. The workspace module (`src/workspace.rs`) already provides equivalent per-ticket file I/O via `read_file`, `write_file`, `list_files` tools. The artifact store is not used in production.

`git2` is the only consumer of the artifact git backend — removing the artifact store removes that dependency entirely.

## Goals / Non-Goals

**Goals:**
- Delete all artifact store code and tests
- Remove artifact-related config fields; reject configs that still include them with a clear error
- Remove `commit_artifact` / `get_artifact` from tool definitions and subagent `VALID_TOOLS`
- Remove `git2` from `Cargo.toml`

**Non-Goals:**
- Migration path for artifact data (unused in practice — none needed)
- Replacing artifact store with anything new
- Touching the workspace module

## Decisions

**Reject `[artifact]` in config rather than silently ignore**

A config with `[artifact]` present should fail validation with: `"[artifact] section is no longer supported; use [workspace] for per-ticket file storage"`. Silent ignore would mask misconfigured files and leave users confused. Loud failure is cheap and honest.

**Remove `max_artifact_inline_bytes` from `LlmConfig`**

This field only existed to cap artifact content inlined into the prompt. With the artifact store gone it has no purpose. Keeping it would be dead config surface with no effect — remove it. Existing configs with this field set will fail TOML deserialization with `unknown field` unless we add `#[serde(deny_unknown_fields)]` — but since we don't use that, TOML will silently ignore unknown fields. No action needed beyond removing it from the struct.

**Remove `git2` entirely**

`git2` is only used in `src/artifact/git.rs`. Removing the module removes the only consumer. This shrinks compile time and removes a large C dependency (libgit2 with vendored OpenSSL).

## Risks / Trade-offs

- **Breaking change for anyone using `[artifact]`** → intentional; config error message directs to workspace
- **`git2` removal** → no other code uses it; verified by grep
- **Test coverage disappears** → all artifact tests deleted; workspace tests already cover the replacement path

## Migration Plan

No data migration. Code deletion only. Config files with `[artifact]` will fail at startup with a clear error.
