## ADDED Requirements

### Requirement: Wizard offers optional artifact store setup
After the board selection step, the wizard SHALL present a confirmation prompt asking whether to configure an artifact store. If the user declines, the wizard SHALL complete normally without writing any `[artifact]` config sections.

#### Scenario: User skips artifact setup
- **WHEN** the user answers "no" to the artifact store prompt
- **THEN** the wizard writes the config without `[artifact]` or `[artifact.git]` sections and prints a success message

#### Scenario: User accepts artifact setup
- **WHEN** the user answers "yes" to the artifact store prompt
- **THEN** the wizard proceeds to the artifact path prompt

### Requirement: Wizard prompts for artifact store local path
The wizard SHALL prompt for a local filesystem path for the artifact git repository. The default SHALL be `~/.orga/artifacts`.

#### Scenario: Default path accepted
- **WHEN** the user accepts the default path
- **THEN** the wizard proceeds using `~/.orga/artifacts`

#### Scenario: Re-run pre-fills existing path
- **WHEN** `[artifact.git].path` already exists in the config
- **THEN** the existing path is shown as the default

### Requirement: Wizard accepts an existing git repo at the given path
If the given path exists and is a valid git repository, the wizard SHALL accept it without performing any git operation.

#### Scenario: Valid existing repo
- **WHEN** the path contains a valid git repository
- **THEN** the wizard reads existing remote and branch configuration from the repo and proceeds to write config

#### Scenario: Path exists but is not a git repo
- **WHEN** the path exists but is not a valid git repository
- **THEN** the wizard returns an error and does not write any config

### Requirement: Wizard offers clone or local-init for missing path
If the given path does not exist, the wizard SHALL prompt for an optional remote URL.

#### Scenario: Remote URL provided — clone performed
- **WHEN** the user enters a remote URL and the path does not exist
- **THEN** the wizard prompts for branch (default: `main`), remote name (default: `origin`), and SSH key path (default: `~/.ssh/id_rsa`), then clones the repository to the given path using the specified key (or the SSH agent if left blank)

#### Scenario: Clone fails
- **WHEN** `git2::Repository::clone` returns an error
- **THEN** the wizard exits with a non-zero code and a descriptive error message; no config is written

#### Scenario: No remote URL — local init performed
- **WHEN** the user leaves the remote URL blank and the path does not exist
- **THEN** the wizard calls `git2::Repository::init` at the given path and writes config without a remote

### Requirement: Wizard writes artifact config sections
On completion of the artifact sub-flow, the wizard SHALL write `[artifact]` and `[artifact.git]` sections to the config file. If an SSH key path was provided, it SHALL be written to `[artifact.git] ssh_key`. No other auth credentials SHALL be written.

#### Scenario: Remote artifact store without explicit SSH key
- **WHEN** artifact setup completes with a remote URL and the SSH key prompt is left blank
- **THEN** the written config contains `backend = "git"`, `path`, `remote`, and `branch` under `[artifact.git]`; no auth fields are present

#### Scenario: Remote artifact store with explicit SSH key
- **WHEN** artifact setup completes with a remote URL and an SSH key path is provided
- **THEN** the written config contains `backend = "git"`, `path`, `remote`, `branch`, and `ssh_key` under `[artifact.git]`

#### Scenario: Local artifact store
- **WHEN** artifact setup completes without a remote URL
- **THEN** the written config contains `backend = "git"` and `path` only; no `remote`, `branch`, or auth fields are present
