## Why

`orga` has no published distribution channel — users must clone and build from source. Making it installable via `mise` with pre-built binaries removes the Rust toolchain requirement and enables one-command installation across all supported platforms.

## What Changes

- Add a GitHub Actions release workflow triggered on `v*` tags
- Build pre-built binaries for all supported targets and package them as archives
- Publish archives as assets on a GitHub Release, enabling `mise use ubi:twistedogic/orga`

## Capabilities

### New Capabilities

- `release-distribution`: GitHub Release workflow that builds, archives, and publishes pre-built binaries for all supported targets on version tag push

### Modified Capabilities

<!-- none -->

## Impact

- New file: `.github/workflows/release.yml`
- No changes to application source code or existing workflows
- Adds `armv7-unknown-linux-gnueabihf` target (cross-compiled) to supported platforms
- Enables `mise use ubi:twistedogic/orga` as the canonical install method
