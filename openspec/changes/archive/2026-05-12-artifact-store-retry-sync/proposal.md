## Why

`artifact list`, `artifact get`, and `artifact commit` operate on local state without syncing from the remote first, meaning agents may read stale artifacts or commit on top of a diverged branch. Adding fetch-before-read and retry-with-backoff on commit makes the artifact store reliable in multi-agent environments.

## What Changes

- `artifact list` and `artifact get` perform a fetch+rebase before reading the filesystem; retries up to 3 times with exponential backoff (100ms, 200ms, 400ms); falls back to local state with a warning if all attempts fail
- `artifact commit` performs a fetch+rebase before writing, then commits and pushes; retries the full cycle (fetch→write→commit→push) up to 3 times with backoff; on push failure each attempt hard-resets HEAD and removes the written file; returns an error after 3 failed attempts
- `fetch_rebase_push` is split into `fetch_rebase` and `push` primitives to support the new retry loops
- Behavior is unchanged when no remote is configured (local-only stores)

## Capabilities

### New Capabilities

- `artifact-store-sync`: Fetch-before-read and retry-with-backoff semantics for the git artifact store

### Modified Capabilities

- `artifact-git-backend`: Retry logic and split sync primitives change the backend's behavior contract

## Impact

- `src/artifact/git.rs` — primary change site; `fetch_rebase_push` split, retry loops added to `list`, `get`, `commit`
- No API or CLI surface changes; behavior difference is observable only via stderr warnings and error messages
- New dependency: `std::thread::sleep` + `std::time::Duration` (stdlib, no new crates)
