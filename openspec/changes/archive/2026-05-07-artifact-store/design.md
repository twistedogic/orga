## Context

Agents produce deliverables as part of ticket work but have no structured way to store them. The existing `MemoryStore` is a private, local, overwrite-in-place scratchpad — not suited for outputs meant to persist, be versioned, or be shared. The `Board` trait follows a clean backend abstraction pattern; the artifact store should follow the same pattern so future backends (e.g., S3, board attachments) can be added without touching the CLI layer.

## Goals / Non-Goals

**Goals:**
- `ArtifactStore` trait with `commit`, `get`, and `list` operations
- `GitArtifactStore` as the pilot backend — dedicated git repo, per-ticket per-agent namespacing, auto-rebase-and-push on commit
- `orga artifact` CLI subcommand (commit, list, get)
- Text content only (inline string or `--file` path)
- Config-driven: `[artifact]` section with backend, path, and remote

**Non-Goals:**
- Binary artifact support (pilot is text-only)
- Board-attached artifacts (Trello card attachments) — future backend
- Artifact deletion or renaming
- Browsing artifact history (git log is the escape hatch)

## Decisions

### D1: Trait-based abstraction mirroring `Board`

The `ArtifactStore` trait lives in `src/artifact/mod.rs` alongside a `build_artifact_store` factory, exactly mirroring `src/board/mod.rs`. This keeps the CLI layer ignorant of the backend and makes future implementations straightforward.

*Alternative considered*: Hardcode git directly in the CLI handler. Rejected — inconsistent with project architecture and forecloses extensibility.

### D2: Dedicated git repo (not inline into project repo)

Artifacts are stored in a separate git repo at a configured path (e.g., `~/.orga/artifacts`). The agent initializes the repo if it doesn't exist.

*Alternative considered*: Commit into the current working directory repo. Rejected — agents work across multiple projects; artifact history should not pollute project history, and the repo path is not knowable at invocation time.

### D3: Per-ticket, per-agent namespace

Files are stored at `artifacts/<ticket-id>/<agent-name>/<artifact-name>`. The agent name comes from `config.agent.name`.

```
artifacts/
  TICKET-123/
    agent-7/
      report.md
      output.json
    agent-9/
      summary.md
```

This eliminates rebase conflicts when multiple agents work on the same ticket. Each agent owns its namespace; no coordination needed.

*Alternative considered*: Flat `artifacts/<ticket-id>/<name>`. Rejected — two agents writing the same artifact name (e.g., `report.md`) would cause git conflicts during rebase.

### D4: Auto-rebase-and-push on every commit

On `artifact commit`, the backend:
1. Writes the file to the working tree
2. `git add`
3. `git commit -m "artifact(<ticket-id>/<agent-name>): <name>"`
4. `git fetch <remote>`
5. `git rebase <remote>/<branch>`
6. `git push <remote> <branch>`

Using `git2` crate for all git operations (no subprocess). Remote is optional — if absent, push step is skipped (local-only mode).

*Alternative considered*: `git merge` instead of rebase. Rejected — rebase keeps a clean linear history suitable for artifact browsing.

*Alternative considered*: Shell out to `git` binary. Rejected — `git2` is already a natural fit for Rust, avoids PATH dependency, and provides structured error handling.

### D5: Text-only content for pilot

`commit` accepts either inline text (`<content>` positional arg) or a file path (`--file <path>`). Content is stored as UTF-8. `get` returns the content as a string.

*Alternative considered*: `Vec<u8>` throughout. Deferred — binary support adds complexity (base64 in JSON output, display issues) with no immediate need.

### D6: Config shape

```toml
[artifact]
backend = "git"

[artifact.git]
path = "~/.orga/artifacts"
remote = "origin"          # optional
branch = "main"            # optional, default "main"
```

Follows the same optional-section pattern as `[memory]` and `[trello]`. If `[artifact]` is absent, artifact commands fail with a clear config error.

## Risks / Trade-offs

- **Rebase failure on push** → If two agents commit the same file path simultaneously, the second push will fail mid-rebase. Mitigation: per-agent namespace makes same-path conflicts impossible between agents; retry once on rebase failure before surfacing the error.
- **Repo not initialized** → If `path` does not exist or is not a git repo, `build_artifact_store` returns a config error. Document that `git init` (and optionally `git remote add origin <url>`) is a one-time setup step; a future `orga artifact init` command could automate this.
- **Remote not configured** → Push step silently skipped. This is intentional for local-only use but could surprise users. Make it explicit in CLI output: "artifact committed (no remote configured, not pushed)".
- **Large files** → Git is not suited for large binary artifacts. Not a concern for pilot (text-only), but worth noting for future backends.

## Open Questions

- Should `orga artifact init` be added to automate repo + remote setup, or is manual `git init` acceptable for the pilot?
- Should `list` show artifacts across all agents for a ticket, or only the current agent's? Current design: all agents (useful for human and agent reviewers).
