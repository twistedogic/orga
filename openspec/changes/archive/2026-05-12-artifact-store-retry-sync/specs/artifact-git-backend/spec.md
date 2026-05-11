## MODIFIED Requirements

### Requirement: Auto-rebase-and-push when remote is configured
When `[artifact.git].remote` is set, `commit` SHALL: fetch from the remote, rebase the current branch onto `<remote>/<branch>`, write the artifact file, create a git commit, then push. If the push fails, the commit SHALL be undone (hard reset to HEAD~1), the artifact file SHALL be removed, and the full cycle SHALL be retried up to 3 times with exponential backoff (100ms, 200ms, 400ms). After 3 failed attempts, `commit` SHALL return an `OrgaError::BackendError`.

#### Scenario: Push succeeds after clean rebase
- **WHEN** remote is configured and no conflicts exist
- **THEN** the commit is pushed to the remote branch after a successful rebase

#### Scenario: Push fails and retries with re-sync
- **WHEN** remote is configured, the first push fails because the remote has diverged, and the second attempt succeeds
- **THEN** the store re-fetches, re-rebases, re-writes the file, re-commits, and pushes successfully

#### Scenario: All retries exhausted
- **WHEN** remote is configured and all 3 push attempts fail
- **THEN** `commit` returns `Err(OrgaError::BackendError)` and the local repository is in a clean state (no uncommitted files, no dangling commits)

#### Scenario: No push when remote is absent
- **WHEN** `[artifact.git].remote` is not configured
- **THEN** the commit is created locally only; no network operations are performed
