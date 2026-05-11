## 1. Refactor sync primitives

- [x] 1.1 Split `fetch_rebase_push` into `fetch_rebase(repo, remote, branch, auth)` and `push(repo, remote, branch, auth)` in `src/artifact/git.rs`
- [x] 1.2 Verify existing `commit` behavior is unchanged by updating the call site to use `fetch_rebase` + `push` in sequence

## 2. Add retry helper

- [x] 2.1 Implement a private `retry_with_backoff` helper (or inline logic) with 3 attempts and delays of 100ms, 200ms, 400ms using `std::thread::sleep` and `std::time::Duration`

## 3. Fetch-before-read for `list` and `get`

- [x] 3.1 Add sync pre-flight to `GitArtifactStore::list`: attempt `fetch_rebase` up to 3 times with backoff; on all failures print a warning to stderr containing "stale" and fall through to local read
- [x] 3.2 Add sync pre-flight to `GitArtifactStore::get`: same retry + fallback behavior as `list`
- [x] 3.3 Skip sync entirely in both methods when `self.remote` is `None`

## 4. Retry loop for `commit`

- [x] 4.1 Wrap the `commit` body in a retry loop (up to 3 attempts): call `fetch_rebase`, write file, create git commit, attempt `push`
- [x] 4.2 On push failure: hard-reset HEAD~1 (`ResetType::Hard` to parent commit), call `fs::remove_file` on the written artifact path, sleep with backoff, then continue to next attempt
- [x] 4.3 After 3 failed push attempts, return `Err(OrgaError::BackendError)` with a descriptive message including the last error
- [x] 4.4 Skip retry loop when `self.remote` is `None` (commit locally only, no push)

## 5. Tests

- [x] 5.1 Test `list` fallback: mock/simulate sync failure (no-remote case as proxy) and verify local results are returned
- [x] 5.2 Test `get` fallback: same approach
- [x] 5.3 Test `commit` cleanup: verify that after a simulated push failure the artifact file does not exist and HEAD is at the pre-commit state
- [x] 5.4 Run full test suite (`cargo test`) and confirm all existing tests still pass
