# init-agent-command Specification

## Purpose
Interactive TUI setup wizard for configuring agent (LLM) mode in the orga config file.

## Requirements

### Requirement: init agent subcommand exists
The CLI SHALL provide `orga init agent` as a subcommand of `orga init` that launches an interactive TUI wizard to create or update the `[llm]`, `[memory]`, `[workspace]`, and `[skills]` sections of the config file. The command SHALL NOT require a valid `[llm]` section to already exist.

#### Scenario: First-time agent setup
- **WHEN** `orga init agent` is run and no `[llm]` section exists
- **THEN** the wizard prompts for provider, API key, and model with no pre-populated values

#### Scenario: Re-run with existing llm config
- **WHEN** `orga init agent` is run and a `[llm]` section already exists
- **THEN** each prompt is pre-populated with the current values from the existing config

#### Scenario: Config path override respected
- **WHEN** `--config <path>` or `ORGA_CONFIG` is set
- **THEN** the wizard reads from and writes to that path

### Requirement: Wizard prompts for LLM provider and model
The wizard SHALL present a `Select` prompt for provider (`anthropic`, `openai`). The model prompt SHALL default to `claude-opus-4-5` when `anthropic` is selected and `gpt-4o` when `openai` is selected. The API key SHALL be prompted as a masked password input.

#### Scenario: Anthropic selected
- **WHEN** the user selects `anthropic`
- **THEN** the model prompt defaults to `claude-opus-4-5`

#### Scenario: OpenAI selected
- **WHEN** the user selects `openai`
- **THEN** the model prompt defaults to `gpt-4o`

#### Scenario: Existing provider pre-selected
- **WHEN** a `[llm]` section already exists with a known provider
- **THEN** that provider is pre-selected in the `Select` prompt

#### Scenario: API key input is masked
- **WHEN** the user enters the LLM API key
- **THEN** the input is hidden/masked and not echoed to the terminal

#### Scenario: Existing API key preserved on blank input
- **WHEN** a `[llm]` section already exists and the user leaves the API key prompt blank
- **THEN** the existing API key is kept

### Requirement: Wizard prompts for optional sections
After the required LLM prompts, the wizard SHALL prompt for optional `[memory]` path, `[workspace]` path, and `[skills]` path. Each prompt SHALL be skippable (empty input = omit the section).

#### Scenario: Memory path provided
- **WHEN** the user enters a memory path
- **THEN** the written config includes `[memory]\npath = "<value>"`

#### Scenario: Memory path skipped
- **WHEN** the user leaves the memory path prompt blank
- **THEN** no `[memory]` section is written (or existing section is preserved if already present)

#### Scenario: Workspace path provided
- **WHEN** the user enters a workspace path
- **THEN** the written config includes `[workspace]\npath = "<value>"`

#### Scenario: Skills path provided
- **WHEN** the user enters a skills path
- **THEN** the written config includes `[skills]\npath = "<value>"`

### Requirement: Wizard merges into existing config
When an existing config file is present, `orga init agent` SHALL preserve all other sections (`[board]`, `[trello]`, `[linear]`, `[[workflow]]`, `[[subagents]]`, etc.) and only update the LLM-related sections.

#### Scenario: Board config preserved after agent init
- **WHEN** a config with `[trello]` and `[board]` exists and `orga init agent` is run
- **THEN** the written config retains `[trello]` and `[board]` unchanged

#### Scenario: No existing config file
- **WHEN** no config file exists at the resolved path
- **THEN** the wizard writes a new file containing only the sections configured during the wizard

### Requirement: Written config self-validates
On completion, the wizard SHALL attempt to load the written config via `AppConfig::load()` and exit with an error if parsing fails.

#### Scenario: Config written successfully
- **WHEN** all prompts are completed
- **THEN** a valid `config.toml` is written and `AppConfig::load()` succeeds

#### Scenario: Config self-validation failure
- **WHEN** the written config fails to parse
- **THEN** the wizard exits with a non-zero code and an error message; the file is left as-written for inspection
