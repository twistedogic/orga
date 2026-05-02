## ADDED Requirements

### Requirement: Init command
The CLI SHALL provide `orga init` as a top-level command (not under any subcommand group) that launches the interactive setup wizard. It SHALL be listed in `orga --help` output.

#### Scenario: Init appears in help
- **WHEN** `orga --help` is run
- **THEN** `init` is listed as an available command with a brief description

#### Scenario: Init does not require existing config
- **WHEN** `orga init` is run before any config file exists
- **THEN** the command starts successfully without a config-not-found error
