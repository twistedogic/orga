## MODIFIED Requirements

### Requirement: Init command runs interactive setup wizard
The CLI SHALL provide `orga init board` and `orga init agent` as subcommands of `orga init`. `orga init` alone SHALL display subcommand help and exit. The commands SHALL NOT require a valid config file to already exist.

#### Scenario: orga init alone shows help
- **WHEN** `orga init` is run without a subcommand
- **THEN** the CLI prints subcommand help listing `board` and `agent` and exits with a non-zero code

#### Scenario: First-time board setup with no existing config
- **WHEN** `orga init board` is run and no config file exists at the resolved path
- **THEN** the wizard starts with empty defaults for all prompts

#### Scenario: Re-run with existing config
- **WHEN** `orga init board` is run and a config file already exists
- **THEN** each prompt is pre-populated with the current value from the existing config

#### Scenario: Config path override respected
- **WHEN** `--config <path>` or `ORGA_CONFIG` is set
- **THEN** the wizard reads from and writes to that path

## REMOVED Requirements

### Requirement: Wizard writes a valid config file
**Reason**: Replaced by `AppConfig::save(path)` which is used by both `init board` and `init agent`. The write behavior is equivalent but no longer uses hand-formatted TOML strings.
**Migration**: No user-visible change. The written file format is identical. The self-validation scenario is preserved in the `init-agent-command` spec and implicitly in `init board` behavior.
