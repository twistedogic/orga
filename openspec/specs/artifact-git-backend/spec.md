# artifact-git-backend Specification

## Purpose
Defines the `GitArtifactStore` backend: stores artifacts in a dedicated git repository, namespaced per ticket and agent, with optional remote rebase-and-push.

## Requirements

### Requirement: Git backend stores artifacts in a dedicated repo
The `GitArtifactStore` SHALL store artifacts in a dedicated git repository at the path configured in `[artifact.git].path`. The repository SHALL NOT be the current working directory or any project repository.

#### Scenario: Artifact written to dedicated repo
- **WHEN** `commit` is called with ticket-id `TICKET-123`, agent name `agent-7`, and name `report.md`
- **THEN** the file is written to `<repo>/artifacts/TICKET-123/agent-7/report.md`

### Requirement: Per-ticket per-agent namespace
Artifacts SHALL be stored at the path `artifacts/<ticket-id>/<agent-name>/<artifact-name>` within the repo. The agent name SHALL be taken from `config.agent.name`.

#### Scenario: Two agents commit same artifact name for same ticket
- **WHEN** agent-7 and agent-9 both commit `report.md` for `TICKET-123`
- **THEN** files are stored at `artifacts/TICKET-123/agent-7/report.md` and `artifacts/TICKET-123/agent-9/report.md` without conflict

### Requirement: Commit on every write
Each `commit` call SHALL create a git commit with message `artifact(<ticket-id>/<agent-name>): <name>`.

#### Scenario: Commit message format
- **WHEN** agent-7 commits `output.json` for `TICKET-123`
- **THEN** the git commit message is `artifact(TICKET-123/agent-7): output.json`

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

### Requirement: Rebase conflict results in error
If the rebase fails due to a conflict, `commit` SHALL return an `OrgaError::BackendError` and leave the repository in a clean state (rebase aborted).

#### Scenario: Rebase conflict
- **WHEN** the remote has diverged in a way that conflicts with the local commit
- **THEN** `commit` aborts the rebase, cleans up, and returns an error

### Requirement: list returns all agents' artifacts for a ticket
`list` SHALL return `ArtifactMeta` for every artifact under `artifacts/<ticket-id>/` regardless of agent.

#### Scenario: Multiple agents' artifacts listed
- **WHEN** `list` is called for `TICKET-123` with artifacts from agent-7 and agent-9
- **THEN** all artifacts from both agents are returned

### Requirement: get retrieves artifact by ticket and name scoped to current agent
`get` SHALL return the artifact at `artifacts/<ticket-id>/<agent-name>/<name>` for the configured agent. Returns `None` if not found.

#### Scenario: Artifact found
- **WHEN** `get` is called with a ticket-id and name that exist for the current agent
- **THEN** returns `Some(Artifact)` with full content

#### Scenario: Artifact not found
- **WHEN** `get` is called with a name that does not exist for the current agent
- **THEN** returns `None`
