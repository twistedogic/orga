## Context

`GitArtifactStore` currently does no pre-flight sync on `list` or `get`, and only syncs on `commit` via a single `fetch_rebase_push` call. In multi-agent deployments, agents sharing the same remote repo can read stale artifacts or race to push, causing commit failures with no recovery path.

The existing `fetch_rebase_push` function bundles fetch, rebase, and push into one call, making it unsuitable for the pre-read path or retry loops that need to interleave write steps between sync and push.

## Goals / Non-Goals

**Goals:**
- `list` and `get` sync from remote before reading; degrade gracefully to local state after 3 failed attempts
- `commit` retries the full cycle (fetch→rebase→write→commit→push) up to 3 times; cleans up (reset + file removal) after each push failure before retrying
- All retries use exponential backoff: 100ms, 200ms, 400ms
- Agents are warned via stderr when falling back to stale local data
- No behavior change when no remote is configured

**Non-Goals:**
- Conflict resolution in rebase (still errors out on conflict)
- Distributed locking or CRDT-style merging
- Retry configuration via config file (hardcoded 3 attempts / backoff for now)
- Changes to `ArtifactStore` trait or CLI surface

## Decisions

### Split `fetch_rebase_push` into `fetch_rebase` + `push`

**Decision**: Replace the single `fetch_rebase_push` function with two separate primitives.

**Rationale**: `commit` needs to interleave write steps between sync and push (fetch→rebase→write file→git commit→push). A single bundled function can't express this. `list` and `get` only need the sync half. Splitting makes both retry loops composable.

**Alternative considered**: Add a `sync_only: bool` parameter — rejected; boolean flags obscure intent and don't help the commit retry loop.

---

### Retry the full commit cycle, not just the push

**Decision**: On push failure in `commit`, reset HEAD~1, remove the written file, sleep, then restart from `fetch_rebase`.

**Rationale**: A push failure means another agent pushed since our rebase. Simply retrying the push won't work — we'd be pushing a diverged ref. We must re-sync, re-apply, re-commit, and re-push. This ensures each attempt starts from a consistent, clean local state.

**Alternative considered**: Retry push only with a force-push — rejected; force-push would silently discard the competing agent's work.

---

### Fall back to local data for `list` / `get` after 3 sync failures

**Decision**: After exhausting retries, print a stderr warning and proceed with local filesystem state.

**Rationale**: Reads are non-destructive. Returning an error on a transient network outage would break agent workflows unnecessarily. A warning gives the agent observability without blocking it.

**Alternative considered**: Hard fail on all sync errors — rejected; too fragile for agents running in CI or flaky network environments.

---

### Backoff sequence: 100ms → 200ms → 400ms

**Decision**: Fixed exponential backoff with no jitter, no configuration.

**Rationale**: With 3 attempts, the total worst-case wait is 700ms — acceptable latency for a CLI tool. The sequence was chosen in the explore phase. Jitter adds complexity for marginal benefit at this scale.

## Risks / Trade-offs

- **Network latency on every read** → `list` and `get` are now network-bound when a remote is configured. Mitigation: the fallback path means a slow/unreachable remote degrades gracefully rather than blocking indefinitely; git fetch is typically fast for small repos.

- **File left on disk after exhausted `commit` retries** → Each retry cleans up with `reset --hard` + `fs::remove_file` before sleeping. If the process is killed mid-cleanup, the file may remain without a commit. Mitigation: next `commit` call will overwrite the file, so it's self-healing on the next invocation.

- **Rebase conflicts still hard-fail** → A genuine conflict (two agents editing the same artifact) will fail on every retry and eventually return an error from `commit`. Mitigation: the namespace scheme (`artifacts/<ticket>/<agent>/`) makes genuine conflicts impossible for the current agent's own files; conflicts only arise if an external party modifies the repo in an unusual way.

## Open Questions

- Should the retry count and backoff be configurable in `[artifact.git]` config in the future?
