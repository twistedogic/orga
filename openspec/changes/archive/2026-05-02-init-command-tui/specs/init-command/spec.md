## ADDED Requirements

### Requirement: Init command runs interactive setup wizard
The CLI SHALL provide `orga init` as a top-level command that launches an interactive TUI wizard to create or update the config file. The command SHALL NOT require a valid config file to already exist.

#### Scenario: First-time setup with no existing config
- **WHEN** `orga init` is run and no config file exists at the resolved path
- **THEN** the wizard starts with empty defaults for all prompts

#### Scenario: Re-run with existing config
- **WHEN** `orga init` is run and a config file already exists
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

### Requirement: Wizard writes a valid config file
On completion the wizard SHALL write a valid TOML config to the resolved path and verify it loads without error.

#### Scenario: Config written successfully
- **WHEN** all prompts are completed and a board is selected
- **THEN** a valid `config.toml` is written, the config directory is created if absent, and a success message including the written path is printed

#### Scenario: Written config self-validates
- **WHEN** the file is written
- **THEN** the wizard attempts to load it via `AppConfig::load()` and exits with an error if parsing fails
