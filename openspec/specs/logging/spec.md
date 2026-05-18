# logging Specification

## Purpose
Persistent file-based logger providing error, warn, and debug levels. Used by all backends and the main error handler to record events across CLI invocations.

## Requirements

### Requirement: Logger struct
The system SHALL provide a `Logger` struct in `src/logging.rs` that writes timestamped log entries to an append-mode file. The logger SHALL support three levels: `error`, `warn`, and `debug`. The file SHALL be opened (or created) in append mode at construction time. If the file cannot be opened, the logger SHALL operate silently without writing (no panic, no error propagation). The `Logger` SHALL be `Send + Sync` via internal `Mutex<Option<File>>`.

#### Scenario: Logger writes error entry
- **WHEN** `logger.error("some message")` is called
- **THEN** a line of the form `2026-01-01T00:00:00Z ERROR some message` is appended to the log file

#### Scenario: Logger writes warn entry
- **WHEN** `logger.warn("some message")` is called
- **THEN** a line of the form `2026-01-01T00:00:00Z WARN  some message` is appended to the log file

#### Scenario: Logger writes debug entry when enabled
- **WHEN** `logger.debug("some message")` is called and `debug` is `true`
- **THEN** a line of the form `2026-01-01T00:00:00Z DEBUG some message` is appended to the log file

#### Scenario: Logger suppresses debug entry when disabled
- **WHEN** `logger.debug("some message")` is called and `debug` is `false`
- **THEN** nothing is written to the log file

#### Scenario: Logger silently no-ops if file cannot be opened
- **WHEN** the log file path is unwritable or invalid
- **THEN** the logger is constructed without error and all log calls silently no-op

### Requirement: Logger timestamp format
Log entry timestamps SHALL use RFC3339 format in UTC (e.g., `2026-05-18T10:23:45Z`). Level labels SHALL be padded to 5 characters (`ERROR`, `WARN `, `DEBUG`).

#### Scenario: Timestamp is UTC RFC3339
- **WHEN** any log entry is written
- **THEN** the timestamp is in UTC RFC3339 format

### Requirement: HTTP error body logging
When Trello returns a 4xx or 5xx HTTP response, the response body SHALL be read and logged at `error` level. The log entry SHALL include the HTTP status code and response body on separate lines. The `OrgaError` returned to the caller SHALL include the status code but not the full body.

#### Scenario: 449 response logged with body
- **WHEN** Trello returns HTTP 449 with a JSON body
- **THEN** the log file contains an entry with the status code and the response body text
- **THEN** the error returned to the caller contains only "Trello returned HTTP 449"

#### Scenario: 4xx with empty body
- **WHEN** Trello returns a 4xx response with no body
- **THEN** the log entry records the status code and an empty body indicator

### Requirement: Fatal error logging
When `exit_error` is called in `main.rs`, the error message SHALL be written to the log file at `error` level before the process exits.

#### Scenario: Fatal error logged before exit
- **WHEN** a command fails and `exit_error` is called
- **THEN** the log file contains an `ERROR` entry with the message before process termination

### Requirement: Warning logging
Non-fatal warnings (e.g., git artifact sync failure) SHALL be written to the log file at `warn` level instead of bare `eprintln!`.

#### Scenario: Git sync warning logged
- **WHEN** the artifact git backend fails to sync and falls back to stale local data
- **THEN** the log file contains a `WARN` entry with the sync failure message
