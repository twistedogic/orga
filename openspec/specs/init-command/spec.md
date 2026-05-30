# init-command Specification

## Purpose
Interactive TUI setup wizard for creating or updating the orga config file.

## Requirements

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

### Requirement: Wizard collects agent name and Trello credentials
The wizard SHALL prompt for agent name (plain text), Trello API key (plain text), and Trello token (masked password input) in that order.

#### Scenario: Token input is masked
- **WHEN** the user enters the Trello token
- **THEN** the input is hidden/masked and not echoed to the terminal

#### Scenario: Existing values shown as defaults
- **WHEN** a config already exists and prompts are shown
- **THEN** the current agent name, API key, and token are offered as default values that the user can accept or overwrite

### Requirement: Wizard auto-fetches member ID
After Trello credentials are entered, the wizard SHALL call `GET https://api.trello.com/1/members/me` and extract the member ID automatically. The member ID SHALL NOT be prompted.

#### Scenario: Successful credential verification
- **WHEN** valid API key and token are entered
- **THEN** the wizard prints "Authenticated as @<username> (<full_name>)" and proceeds

#### Scenario: Invalid credentials
- **WHEN** the Trello API returns 401
- **THEN** the wizard exits with a non-zero code and an error message indicating the credentials are invalid; no config file is written

### Requirement: Wizard presents a board picker
After successful authentication, the wizard SHALL call `GET https://api.trello.com/1/members/me/boards` and present the results as a selectable list. The user SHALL pick one board; its ID is stored in the config.

#### Scenario: Boards fetched and displayed
- **WHEN** authentication succeeds
- **THEN** a `Select` prompt shows all boards by name; selection stores the corresponding board ID

#### Scenario: Existing board pre-selected
- **WHEN** a config already exists and its board ID matches one of the fetched boards
- **THEN** that board is pre-selected as the default in the selection list

#### Scenario: No boards found
- **WHEN** the authenticated member has no boards
- **THEN** the wizard exits with a non-zero code and a message that no boards were found
