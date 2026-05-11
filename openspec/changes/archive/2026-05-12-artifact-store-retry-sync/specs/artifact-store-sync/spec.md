## ADDED Requirements

### Requirement: Fetch-before-read syncs from remote before list and get
When a remote is configured, `list` and `get` SHALL perform a fetch+rebase from the remote before reading the local filesystem. The sync SHALL be attempted up to 3 times with exponential backoff (100ms, 200ms, 400ms) before falling back to local state.

#### Scenario: Sync succeeds before list
- **WHEN** `list` is called and a remote is configured
- **THEN** the store fetches and rebases from the remote before returning results

#### Scenario: Sync succeeds before get
- **WHEN** `get` is called and a remote is configured
- **THEN** the store fetches and rebases from the remote before returning the artifact

#### Scenario: Sync fails and falls back to local for list
- **WHEN** `list` is called, a remote is configured, and all 3 sync attempts fail
- **THEN** the store prints a warning to stderr containing the text "stale" and returns local filesystem state as `Ok`

#### Scenario: Sync fails and falls back to local for get
- **WHEN** `get` is called, a remote is configured, and all 3 sync attempts fail
- **THEN** the store prints a warning to stderr containing the text "stale" and returns local filesystem state as `Ok`

#### Scenario: No sync when remote absent for reads
- **WHEN** `list` or `get` is called and no remote is configured
- **THEN** no network operations are performed and the local filesystem is read directly

### Requirement: Commit retries full cycle on push failure
When a remote is configured, `commit` SHALL retry the full fetch→rebase→write→commit→push cycle up to 3 times with exponential backoff (100ms, 200ms, 400ms). On each push failure the local commit SHALL be undone (hard reset to HEAD~1) and the written artifact file SHALL be removed before the next attempt.

#### Scenario: Commit succeeds on first attempt
- **WHEN** `commit` is called and the push succeeds on the first try
- **THEN** returns `Ok(ArtifactMeta)` with no retries

#### Scenario: Commit succeeds on retry
- **WHEN** `commit` is called, the first push fails (remote has diverged), and the second attempt succeeds
- **THEN** returns `Ok(ArtifactMeta)` after re-syncing and re-committing

#### Scenario: Commit fails after 3 attempts
- **WHEN** `commit` is called and all 3 push attempts fail
- **THEN** returns `Err(OrgaError::BackendError)` describing the failure

#### Scenario: File cleaned up after failed push
- **WHEN** a push fails during `commit`
- **THEN** the git commit is undone (HEAD returns to pre-commit state) and the artifact file is removed from disk before the next retry

#### Scenario: No retry when remote absent for commit
- **WHEN** `commit` is called and no remote is configured
- **THEN** the artifact is written and committed locally without any push or retry logic
