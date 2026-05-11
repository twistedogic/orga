## Context

`orga init` is the setup wizard for the CLI. It currently handles Trello authentication and board selection, writing `[agent]`, `[board]`, and `[trello]` sections to the config file. The `[artifact]` section — required for artifact commands — is entirely absent from the wizard; agents must hand-edit TOML to configure it.

The artifact store uses `git2` (already a dependency) and supports a git backend with an optional remote. SSH agent auth is the default path; HTTP credentials are a manual TOML concern and out of scope for the wizard.

## Goals / Non-Goals

**Goals:**
- Add an optional artifact store setup phase to `run_init` after the existing Trello/board steps
- On first run: prompt for local path, offer to clone from URL or init locally
- On re-run: pre-fill existing `[artifact.git]` values as defaults
- Perform the git operation (clone or init) during `init`, not deferred to first use
- Write `[artifact]` + `[artifact.git]` sections to the config file on completion

**Non-Goals:**
- HTTP credential prompts (manual TOML only)
- Explicit SSH key path prompts (SSH agent is used implicitly; no config written)
- Validating remote reachability beyond what `git2::Repository::clone` provides
- Multiple artifact backends (only `git` exists)

## Decisions

### Decision: Opt-in gate with "skip" prompt

An explicit "Configure artifact store? (y/n)" prompt gates the entire sub-flow. Skipping leaves any existing `[artifact]` config intact (re-run safety) or writes nothing (first run). This keeps the wizard fast for agents that don't need artifact storage.

_Alternative considered_: Always show the artifact prompts. Rejected — artifact store is optional in the data model and not all agents need it.

### Decision: Path-state detection drives the flow branch

After the user enters a path, the wizard inspects it:

```
path missing or empty dir  →  ask "Remote URL? (blank = local-only)"
  blank URL   →  git2::Repository::init
  URL given   →  ask branch [main], remote name [origin] → git2::Repository::clone
path is valid git repo     →  accept as-is, skip clone/init
path exists but not a repo →  return error, do not proceed
```

This avoids a separate "local or remote?" question — the path state implies the answer naturally on first run, and on re-run an existing repo skips everything.

### Decision: SSH agent only; no credentials written to config

`git2` callbacks fall through to `Cred::ssh_key_from_agent` when no explicit key is configured. The written config contains only `path`, `remote`, and `branch` — no auth fields. HTTP auth remains a manual TOML concern documented in the README.

_Alternative considered_: Offer an auth type selector (SSH/HTTP). Rejected per explicit design decision — adds prompt complexity for an uncommon case.

### Decision: `write_config_file` extended with optional artifact params

The existing `write_config_file` function is extended to accept an `Option<ArtifactGitConfig>` parameter. When `Some`, it appends the `[artifact]` and `[artifact.git]` TOML blocks. When `None` (user skipped), the blocks are omitted and any previously written artifact config is not preserved.

_Alternative considered_: Read-modify-write the existing TOML. Rejected — the current approach rewrites the whole file from scratch; preserving that invariant is simpler and avoids partial-update bugs.

### Decision: Clone/init happens during `init`, not lazily

The git operation runs immediately when the user completes the artifact sub-flow. Errors (bad URL, SSH key not loaded, path permission denied) surface interactively with a clear message rather than failing silently on first `artifact commit`.

## Risks / Trade-offs

- **SSH agent not running** → `git2` clone will fail with a credential error. Mitigation: print a clear message suggesting the user ensure their SSH agent is running and has the key loaded, then re-run `init`.
- **Config rewrite drops HTTP credentials** — if an existing config has `http_username`/`http_password` in `[artifact.git]` and the user re-runs `init` and confirms the artifact setup, those fields will be lost (the wizard never writes them). Mitigation: document this clearly; HTTP users should not re-run `init` for the artifact section.
- **Large repo clone is slow** — `init` is interactive and blocking; a slow clone blocks the terminal. No mitigation planned; this is acceptable for a one-time setup command.
