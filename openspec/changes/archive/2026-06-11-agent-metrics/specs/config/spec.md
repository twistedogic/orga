## MODIFIED Requirements

### Requirement: Metrics config section
The `AppConfig` SHALL accept an optional `[metrics]` section with a single field `listen_addr` (string, default `"127.0.0.1:9090"`). The section SHALL be optional; absence means metrics are not exposed. The field SHALL be validated to be a valid `host:port` string at config-load time.

#### Scenario: Metrics section absent
- **WHEN** the config file does not contain `[metrics]`
- **THEN** `AppConfig::metrics_config()` returns `None` and no metrics endpoint is bound

#### Scenario: Metrics section with default addr
- **WHEN** the config contains `[metrics]` with no `listen_addr` field
- **THEN** `listen_addr` defaults to `"127.0.0.1:9090"`

#### Scenario: Invalid listen_addr rejected
- **WHEN** the config contains `[metrics] listen_addr = "not-a-socket-addr"`
- **THEN** `AppConfig::load` returns an `OrgaError::ConfigError` mentioning the invalid listen address
