## ADDED Requirements

### Requirement: whoami command
The CLI SHALL provide `orga whoami` as a top-level command. It SHALL call the backend to resolve the configured agent's member profile and output `id`, `username`, and `full_name`.

#### Scenario: Default human-readable output
- **WHEN** `orga whoami` is run without `--json`
- **THEN** the agent's `username` and `full_name` are printed to stdout

#### Scenario: JSON output
- **WHEN** `orga whoami --json` is run
- **THEN** output is a JSON object with fields `id`, `username`, `full_name`

#### Scenario: Invalid credentials
- **WHEN** the configured API key or token is invalid
- **THEN** the command exits with a non-zero code and prints an authorization error to stderr
