## MODIFIED Requirements

### Requirement: Wizard writes a valid config file
On completion the wizard SHALL write a valid TOML config to the resolved path and verify it loads without error. When the user completes the artifact store sub-flow, the config SHALL include `[artifact]` and `[artifact.git]` sections. When the user skips artifact setup, those sections SHALL be omitted.

#### Scenario: Config written successfully without artifact setup
- **WHEN** all Trello/board prompts are completed and the user skips artifact setup
- **THEN** a valid `config.toml` is written containing `[agent]`, `[board]`, and `[trello]` sections; no artifact sections are present

#### Scenario: Config written successfully with artifact setup
- **WHEN** all prompts including artifact store are completed
- **THEN** a valid `config.toml` is written containing `[agent]`, `[board]`, `[trello]`, `[artifact]`, and `[artifact.git]` sections

#### Scenario: Written config self-validates
- **WHEN** the file is written
- **THEN** the wizard attempts to load it via `AppConfig::load()` and exits with an error if parsing fails
