## Context

`orga agent` runs as an event-driven polling loop, intended to be deployed as a persistent background service. Currently, users must manually author a systemd unit file and reload the daemon — a tedious, error-prone step with no canonical reference for the correct binary path, config path, or restart policy.

A new `orga systemd install` command generates the unit file from live runtime values (current executable path, resolved config path) and places it in the correct systemd directory, reducing setup to a single command.

## Goals / Non-Goals

**Goals:**
- Generate a correct `orga-agent.service` unit file using the running binary's path and resolved config path
- Place the unit at `~/.config/systemd/user/` (default) or `/etc/systemd/system/` (`--system`)
- Run `systemctl [--user] daemon-reload` after placement
- Print clear next-step instructions for enabling and starting the service
- Error immediately if `--system` is requested but the process is not root
- Error clearly on non-Linux platforms

**Non-Goals:**
- `systemctl enable` or `systemctl start` — user decides when to activate
- `orga systemd uninstall` — out of scope for this change
- macOS launchd support
- Configuring environment variables in the unit (API keys live in config.toml)

## Decisions

### New module: `src/systemd.rs`
All unit file generation and placement logic lives in a new `systemd` module rather than inline in `main.rs`. This keeps `main.rs` as a thin dispatch layer and makes the logic independently testable.

**Alternative**: inline in `main.rs` — rejected, too much logic for a dispatch file.

### Runtime platform check, not compile-time
Use a runtime `cfg!(target_os = "linux")` check with a clear error message rather than `#[cfg(target_os = "linux")]` attribute gating. This ensures the command always appears in `--help` on all platforms and produces a helpful error rather than a missing-command surprise.

### `std::env::current_exe()` for binary path
The binary path in `ExecStart` is derived from `std::env::current_exe()` rather than asking the user. This is always correct at install time (the running binary is the one being installed).

### No new dependencies
Unit file generation is string formatting; placement is `std::fs::write`; daemon-reload is `std::process::Command::new("systemctl")`. No new crate dependencies needed.

### User vs system service via `--system` flag
Default is user-level service. `--system` opts into system-level. Root check uses `nix`-free approach: check `std::env::var("USER") == "root"` or `unsafe { libc::getuid() } == 0`. Since we don't have `libc` as a dependency, we use `id -u` via `std::process::Command` or check if we can write to `/etc/systemd/system/` — simplest: attempt to open the target path for writing and surface the OS permission error naturally, but per spec we must error *before* attempting. Use `std::env::var("EUID")` fallback or spawn `id -u`.

**Decision**: Use `std::process::Command::new("id").arg("-u")` to get effective UID. Zero = root. This avoids adding `libc` or `nix` as a dependency.

## Risks / Trade-offs

- **`current_exe()` symlink resolution**: on some systems this returns the symlink, not the real path. `std::fs::canonicalize` applied to the result ensures the real path. → Mitigation: always canonicalize.
- **daemon-reload failure**: if `systemctl` is not in PATH (container, CI, non-systemd Linux), daemon-reload will fail. → Mitigation: treat daemon-reload failure as a warning, not a fatal error; print the command and suggest running it manually.
- **Existing unit file**: if `orga-agent.service` already exists, the command overwrites it silently. → Acceptable for now; file is fully regenerated from current state.

## Open Questions

None — scope is fully defined.
