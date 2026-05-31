## ADDED Requirements

### Requirement: Systemd subcommand
The CLI SHALL expose a top-level `systemd` subcommand with an `install` sub-subcommand. The `systemd install` command SHALL accept an optional `--system` flag. The global `--config` flag SHALL be respected to resolve the config path written into the generated unit file.

#### Scenario: Systemd install appears in help
- **WHEN** `orga --help` is run
- **THEN** `systemd` appears as a top-level subcommand in the output

#### Scenario: Install subcommand appears in help
- **WHEN** `orga systemd --help` is run
- **THEN** `install` appears as a subcommand with a description

#### Scenario: --system flag accepted
- **WHEN** `orga systemd install --system` is invoked
- **THEN** the command proceeds with system-level placement logic
