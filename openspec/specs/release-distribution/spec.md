## ADDED Requirements

### Requirement: Release workflow triggers on version tags
The CI system SHALL trigger a release build and publish workflow when a git tag matching `v*` is pushed to the repository.

#### Scenario: Version tag push triggers release
- **WHEN** a tag matching `v*` is pushed to the repository
- **THEN** the release workflow runs and builds binaries for all supported targets

#### Scenario: Non-tag push does not trigger release
- **WHEN** a commit is pushed to a branch without a matching tag
- **THEN** the release workflow does NOT run

### Requirement: Pre-built binaries published for all supported targets
The release workflow SHALL build and publish pre-built binary archives for the following targets:
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `armv7-unknown-linux-gnueabihf`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

#### Scenario: All targets produce release assets
- **WHEN** the release workflow completes successfully
- **THEN** a GitHub Release exists with one archive asset per supported target

### Requirement: Binary archives use platform-appropriate format
Unix targets (Linux, macOS) SHALL be archived as `.tar.gz`. The Windows target SHALL be archived as `.zip`.

#### Scenario: Unix archive format
- **WHEN** a release is published for a Linux or macOS target
- **THEN** the asset is a `.tar.gz` file containing the `orga` binary

#### Scenario: Windows archive format
- **WHEN** a release is published for the Windows target
- **THEN** the asset is a `.zip` file containing `orga.exe`

### Requirement: Asset names include the target triple
Release asset filenames SHALL follow the pattern `orga-<target>.(tar.gz|zip)` where `<target>` is the full Rust target triple (e.g., `aarch64-apple-darwin`).

#### Scenario: Asset filename matches ubi discovery pattern
- **WHEN** a user runs `mise use ubi:twistedogic/orga` on a supported platform
- **THEN** `ubi` resolves the correct asset by matching the platform string in the filename

### Requirement: Cross-compilation used for ARM targets
Targets that cannot be natively compiled on the CI runner (`aarch64-unknown-linux-gnu`, `armv7-unknown-linux-gnueabihf`) SHALL use `cross` for compilation.

#### Scenario: ARM Linux targets build via cross
- **WHEN** the release workflow builds for `aarch64-unknown-linux-gnu` or `armv7-unknown-linux-gnueabihf`
- **THEN** the binary is produced using `cross build --release --target <target>`
