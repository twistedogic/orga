## Context

`orga` is a Rust CLI with no published distribution channel. Users must have a Rust toolchain installed and build from source. The existing CI workflow (`build.yml`) already builds binaries for 5 targets on every push and PR — but those artifacts are ephemeral and not published anywhere. The goal is to publish versioned GitHub Releases with downloadable binary archives so `mise` can install `orga` via its `ubi` backend.

## Goals / Non-Goals

**Goals:**
- Publish pre-built binary archives as GitHub Release assets on every version tag
- Support all current build targets plus `armv7-unknown-linux-gnueabihf` (Raspberry Pi 3B)
- Enable `mise use ubi:twistedogic/orga` as the canonical install method

**Non-Goals:**
- Publishing to crates.io
- Custom mise plugin or aqua registry entry
- Homebrew formula or other package manager support
- Changing application source code

## Decisions

### GitHub Actions release workflow (not reusing build.yml)

A separate `release.yml` triggered on `push: tags: v*` keeps release concerns isolated from CI. The build matrix is duplicated but this is intentional — release builds may diverge (e.g., strip symbols, add checksums) without affecting CI.

**Alternative considered**: Extending `build.yml` with a conditional release job. Rejected because tag-triggered behavior mixed into a push/PR workflow adds complexity and risk of accidental releases.

### Archive formats: `.tar.gz` for Unix, `.zip` for Windows

`ubi` supports both compressed archives and raw binaries. Archives reduce download size and are the conventional format. `.tar.gz` is standard on Linux/macOS; `.zip` is native on Windows.

**Alternative considered**: Raw binaries only. Simpler but unconventional and larger transfers.

### Asset naming: `orga-<target>.(tar.gz|zip)`

`ubi` matches assets by looking for the platform/arch string in the filename. Using the full Rust target triple (e.g., `aarch64-apple-darwin`) is unambiguous and directly supported by `ubi`.

### `armv7-unknown-linux-gnueabihf` via `cross`

The existing matrix already uses `cross` for `aarch64-unknown-linux-gnu`. Adding `armv7` follows the same pattern — no new infra required, just another matrix entry.

### Release creation with `gh release create`

Using `softprops/action-gh-release` or `gh release create --generate-notes` produces a clean release with auto-generated changelog from commit messages. `GITHUB_TOKEN` is sufficient — no extra secrets needed.

## Risks / Trade-offs

- **Cross-compilation flakiness** → `cross` is well-maintained; existing `aarch64` usage validates the approach
- **Tag accidentally triggers on non-release tags** → Mitigated by convention: only `v*` tags trigger the workflow; document this in repo

## Migration Plan

1. Merge `release.yml` to `main`
2. Push a `v0.1.0` tag
3. Verify release assets appear on GitHub
4. Confirm `mise use ubi:twistedogic/orga` installs correctly

No rollback needed — this is purely additive.

## Open Questions

None.
