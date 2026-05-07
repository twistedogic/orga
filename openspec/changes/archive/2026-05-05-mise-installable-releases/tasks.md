## 1. Release Workflow

- [x] 1.1 Create `.github/workflows/release.yml` triggered on `push: tags: v*`
- [x] 1.2 Add build matrix with all 6 targets (x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, armv7-unknown-linux-gnueabihf, x86_64-apple-darwin, aarch64-apple-darwin, x86_64-pc-windows-msvc)
- [x] 1.3 Mark `aarch64-unknown-linux-gnu` and `armv7-unknown-linux-gnueabihf` as `cross: true` in the matrix
- [x] 1.4 Add step to install `cross` for cross-compiled targets

## 2. Archive Packaging

- [x] 2.1 Add step to package Unix binaries as `.tar.gz` (filename: `orga-<target>.tar.gz`)
- [x] 2.2 Add step to package Windows binary as `.zip` (filename: `orga-x86_64-pc-windows-msvc.zip`)

## 3. GitHub Release Publishing

- [x] 3.1 Add step using `softprops/action-gh-release` (or `gh release create`) to create a GitHub Release from the tag
- [x] 3.2 Configure step to upload all archive assets to the release
- [x] 3.3 Verify `GITHUB_TOKEN` permissions are sufficient (contents: write)

## 4. Validation

- [x] 4.1 Push a `v0.1.0` tag and confirm the workflow runs successfully
- [x] 4.2 Confirm all 6 archive assets appear on the GitHub Release
- [x] 4.3 Confirm `mise use ubi:twistedogic/orga` installs the correct binary on the local machine
