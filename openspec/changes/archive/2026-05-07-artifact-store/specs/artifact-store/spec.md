## ADDED Requirements

### Requirement: ArtifactStore trait
The system SHALL define an `ArtifactStore` trait in `src/artifact/mod.rs` with three methods: `commit`, `get`, and `list`. All methods SHALL return `Result<_, OrgaError>`.

#### Scenario: Trait is backend-agnostic
- **WHEN** a new backend implements `ArtifactStore`
- **THEN** it can be used by the CLI without any changes to command dispatch

### Requirement: ArtifactMeta model
The system SHALL define an `ArtifactMeta` struct containing `ticket_id`, `agent_name`, `name`, and `committed_at` (UTC timestamp).

#### Scenario: Meta returned from commit
- **WHEN** `commit` succeeds
- **THEN** it returns an `ArtifactMeta` with all fields populated

### Requirement: Artifact model
The system SHALL define an `Artifact` struct containing all `ArtifactMeta` fields plus `content` (UTF-8 string).

#### Scenario: Content accessible after get
- **WHEN** `get` returns `Some(artifact)`
- **THEN** `artifact.content` contains the full committed text

### Requirement: build_artifact_store factory
The system SHALL provide a `build_artifact_store` factory function that reads `AppConfig` and returns a `Box<dyn ArtifactStore>`. It SHALL fail with a config error if the `[artifact]` section is absent.

#### Scenario: Missing artifact config
- **WHEN** `[artifact]` is absent from config
- **THEN** `build_artifact_store` returns an `OrgaError::ConfigError`

#### Scenario: Unknown artifact backend
- **WHEN** `artifact.backend` is set to an unrecognized value
- **THEN** `build_artifact_store` returns an `OrgaError::ConfigError` listing supported backends
