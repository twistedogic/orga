## 1. CLI Structure

- [x] 1.1 Add `SystemdCommands` enum with `Install { system: bool }` variant to `src/main.rs`
- [x] 1.2 Add `Commands::Systemd(SystemdCommands)` variant to the top-level `Commands` enum
- [x] 1.3 Wire `Commands::Systemd` dispatch in `run_sync` to call the new systemd module

## 2. Systemd Module

- [x] 2.1 Create `src/systemd.rs` with a `install_service(system: bool, config_path: &str) -> Result<(), OrgaError>` function
- [x] 2.2 Implement platform check: return `OrgaError` with clear message on non-Linux
- [x] 2.3 Implement root check for `--system`: spawn `id -u`, parse output, error if non-zero
- [x] 2.4 Implement unit file generation: use `std::env::current_exe()` + `canonicalize()` for binary path, format unit file string with correct `WantedBy` per user/system mode
- [x] 2.5 Implement directory creation and file placement: `fs::create_dir_all` + `fs::write` to target path
- [x] 2.6 Implement `systemctl [--user] daemon-reload` via `std::process::Command`; treat failure as warning, not error
- [x] 2.7 Print success output: written path and next-step `systemctl [--user] enable orga-agent` instruction

## 3. Error Handling

- [x] 3.1 Add `SystemdNotLinux`, `SystemdRootRequired`, and `SystemdWriteFailed` variants to `OrgaError` (or reuse `Io` variant with context)
- [x] 3.2 Expose `systemd` module from `src/lib.rs`

## 4. Tests

- [x] 4.1 Unit test: `generate_unit_file` (pure function) produces correct content for user vs system mode
- [x] 4.2 Unit test: root check logic returns correct error for non-root in system mode
