## ADDED Requirements

### Requirement: AppConfig supports serialization
`AppConfig` and all its sub-structs SHALL derive `serde::Serialize`. All `Option<_>` fields SHALL be annotated with `#[serde(skip_serializing_if = "Option::is_none")]`. All `Vec<_>` fields that have `#[serde(default)]` SHALL additionally have `#[serde(skip_serializing_if = "Vec::is_empty")]`. This enables round-trip TOML serialization without emitting null or empty fields.

#### Scenario: Config round-trips through serialize/deserialize
- **WHEN** a valid `AppConfig` is serialized via `toml::to_string` and the result is deserialized via `AppConfig::load`
- **THEN** the resulting config is equivalent to the original

#### Scenario: None fields omitted from serialized output
- **WHEN** a config with `trello = None` is serialized
- **THEN** the output TOML does not contain a `[trello]` section

#### Scenario: Empty vec fields omitted from serialized output
- **WHEN** a config with an empty `subagents` vec is serialized
- **THEN** the output TOML does not contain any `[[subagents]]` entries

### Requirement: AppConfig provides a save method
`AppConfig` SHALL provide a `save(path: &Path) -> Result<(), OrgaError>` method that serializes the config to TOML and writes it to the given path, creating parent directories as needed.

#### Scenario: Save writes valid TOML
- **WHEN** `config.save(path)` is called on a valid `AppConfig`
- **THEN** the file at `path` contains valid TOML that can be loaded by `AppConfig::load`

#### Scenario: Save creates parent directories
- **WHEN** the parent directory of `path` does not exist
- **THEN** `save` creates it before writing

#### Scenario: Save fails gracefully on write error
- **WHEN** the path is not writable
- **THEN** `save` returns `Err(OrgaError::ConfigError(...))`
