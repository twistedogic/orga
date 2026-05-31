# systemd-install Specification

## Requirements

### Requirement: Systemd install command
The CLI SHALL provide `orga systemd install` to generate an `orga-agent.service` systemd unit file and place it in the appropriate systemd directory. By default it SHALL place a user-level service at `~/.config/systemd/user/orga-agent.service`. With `--system`, it SHALL place a system-level service at `/etc/systemd/system/orga-agent.service`. After placing the file, the command SHALL run `systemctl [--user] daemon-reload`. On success, the command SHALL print the path of the written file and the `systemctl enable` command the user should run next.

#### Scenario: User-level install (default)
- **WHEN** `orga systemd install` is run without `--system`
- **THEN** `~/.config/systemd/user/orga-agent.service` is written with a valid unit file, `systemctl --user daemon-reload` is executed, and next-step instructions are printed to stdout

#### Scenario: System-level install
- **WHEN** `orga systemd install --system` is run as root
- **THEN** `/etc/systemd/system/orga-agent.service` is written with a valid unit file, `systemctl daemon-reload` is executed (without `--user`), and next-step instructions are printed to stdout

#### Scenario: System-level install without root
- **WHEN** `orga systemd install --system` is run as a non-root user
- **THEN** the command exits immediately with a non-zero code and prints an error to stderr indicating root is required

#### Scenario: Non-Linux platform
- **WHEN** `orga systemd install` is run on a non-Linux operating system
- **THEN** the command exits with a non-zero code and prints an error to stderr indicating systemd is only supported on Linux

#### Scenario: daemon-reload unavailable
- **WHEN** `systemctl` is not found in PATH after the unit file is written
- **THEN** the unit file is kept, a warning is printed to stderr, and the command exits with code 0 with instructions to run `daemon-reload` manually

### Requirement: Generated unit file content
The generated `orga-agent.service` unit file SHALL use the absolute path of the running `orga` binary as `ExecStart`, append `--config <resolved-config-path> agent` to the exec line, set `Restart=on-failure`, `RestartSec=30`, and use `WantedBy=default.target` for user services or `WantedBy=multi-user.target` for system services.

#### Scenario: Binary path in unit file
- **WHEN** a unit file is generated
- **THEN** the `ExecStart` line contains the canonicalized absolute path of the currently running `orga` binary

#### Scenario: Config path in unit file
- **WHEN** a unit file is generated with a non-default config path (via `--config`)
- **THEN** the `ExecStart` line includes `--config <absolute-config-path>`

#### Scenario: Restart policy
- **WHEN** a unit file is generated
- **THEN** the `[Service]` section contains `Restart=on-failure` and `RestartSec=30`

#### Scenario: User service WantedBy
- **WHEN** a user-level unit file is generated
- **THEN** the `[Install]` section contains `WantedBy=default.target`

#### Scenario: System service WantedBy
- **WHEN** a system-level unit file is generated
- **THEN** the `[Install]` section contains `WantedBy=multi-user.target`
