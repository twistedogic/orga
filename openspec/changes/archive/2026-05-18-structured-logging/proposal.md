## Why

When Trello returns non-2xx HTTP responses (notably 449 "Retry With"), orga currently discards the response body and surfaces only the status code. For an agent invoking orga as a subprocess, this makes it impossible to diagnose API failures without additional tooling. A persistent log file gives operators and agents a complete, timestamped record of errors, warnings, and debug traces across invocations.

## What Changes

- New `Logger` struct in `src/logging.rs` — append-mode file writer with `error`, `warn`, and `debug` levels; thread-safe via `Mutex<File>`
- `[logging]` config section added to `AppConfig` — optional `file` path (default `~/.orga/orga.log`) and `debug` boolean (default `false`)
- Trello HTTP error handling restructured: response body is captured and logged for all 4xx/5xx responses; error message includes status code; full body goes to the log file
- Non-fatal git artifact sync warning migrated from `eprintln!` to `logger.warn()`
- Fatal errors in `exit_error` logged via `logger.error()` before process exit
- `TrelloBackend` and artifact git backend receive `Arc<Logger>` at construction time

## Capabilities

### New Capabilities

- `logging`: Persistent file-based logger with configurable path and debug level; used by all backends and the main error handler

### Modified Capabilities

- `config`: New optional `[logging]` section with `file` and `debug` fields
- `trello-backend`: HTTP error handling now captures and logs response body for all 4xx/5xx; `check_status` restructured to consume response

## Impact

- `src/logging.rs` — new file
- `src/config.rs` — `LoggingConfig` struct, `AppConfig::logger()` factory
- `src/main.rs` — logger init, pass `Arc<Logger>` to backends, use in `exit_error`
- `src/board/trello.rs` — `Arc<Logger>` field, `check_status` → `handle_response` consuming response body
- `src/artifact/git.rs` — `Arc<Logger>` field, `eprintln!` warning replaced
- No new crate dependencies (uses `std::fs`, `std::io`, `std::sync::Mutex`)
