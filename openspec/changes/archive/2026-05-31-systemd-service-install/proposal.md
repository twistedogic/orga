## Why

Running `orga agent` as a persistent background service requires manual systemd unit authoring, which is error-prone and inconsistent across deployments. A built-in install command makes daemon setup a single step and ensures the unit file is always generated correctly from the current binary and config paths.

## What Changes

- Add `orga systemd install` subcommand that generates and places a systemd service unit file for `orga agent`
- Supports user-level service (`~/.config/systemd/user/`) by default
- Supports system-level service (`/etc/systemd/system/`) via `--system` flag
- Errors immediately if `--system` is requested and the process is not running as root
- Runs `systemctl [--user] daemon-reload` after placing the file
- Prints next-step instructions (`systemctl [--user] enable orga-agent`) to stdout
- Linux-only; emits a clear error on non-Linux platforms

## Capabilities

### New Capabilities
- `systemd-install`: `orga systemd install` command that generates and places an `orga-agent.service` unit file, with support for user vs system service placement

### Modified Capabilities
- `cli-commands`: New top-level `systemd` subcommand with `install` sub-subcommand added to the CLI command tree

## Impact

- `src/main.rs` — new `SystemdCommands` enum and `Commands::Systemd` variant
- New `src/systemd.rs` module implementing unit file generation and placement
- Linux-only logic gated at runtime (clear error on unsupported platforms)
- No new dependencies required (`std::fs`, `std::process::Command` for systemctl)
