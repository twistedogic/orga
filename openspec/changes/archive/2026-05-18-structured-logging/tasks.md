## 1. Logger Module

- [x] 1.1 Create `src/logging.rs` with `Logger` struct holding `Mutex<Option<File>>` and `debug: bool`
- [x] 1.2 Implement `Logger::new(path: &Path, debug: bool) -> Logger` — open file in append mode, silently use `None` on failure
- [x] 1.3 Implement `logger.error(msg: &str)`, `logger.warn(msg: &str)`, `logger.debug(msg: &str)` — write `YYYY-MM-DDTHH:MM:SSZ LEVEL  msg\n`; debug suppressed when disabled
- [x] 1.4 Export `Logger` from `src/lib.rs`

## 2. Config

- [x] 2.1 Add `LoggingConfig` struct to `src/config.rs` with optional `file: Option<String>` and `debug: Option<bool>`
- [x] 2.2 Add `pub logging: Option<LoggingConfig>` field to `AppConfig`
- [x] 2.3 Add `AppConfig::logger(&self) -> Logger` factory method — resolves default path `~/.orga/orga.log` if not configured, resolves `~` via `expand_tilde`, passes `debug` flag
- [x] 2.4 Add config tests: logging section present, absent (defaults applied), debug flag propagated

## 3. Trello Backend

- [x] 3.1 Add `logger: Arc<Logger>` field to `TrelloBackend`
- [x] 3.2 Update `TrelloBackend::new` signature to accept `Arc<Logger>`
- [x] 3.3 Replace `check_status` with `handle_response`(&self, resp: Response) -> Result<String, OrgaError>` — consumes response, reads body as text, logs at error level for 4xx/5xx, returns body string on success
- [x] 3.4 Update `get` helper: call `handle_response(resp)?` instead of `check_status(&resp)?` + `resp.json()`; deserialize from returned string via `serde_json::from_str`
- [x] 3.5 Update `post_form` helper: same pattern as `get`
- [x] 3.6 Update `put_form` helper: same pattern
- [x] 3.7 Update the 4 direct `.send()` call sites in the `Board` impl (`list_assigned`, `get_ticket`, `whoami`, and any remaining) to use `handle_response`
- [x] 3.8 Verify all existing status-specific mappings (429→RateLimited, 401→Unauthorized, 404→NotFound) are preserved in `handle_response`

## 4. Artifact Git Backend

- [x] 4.1 Add `logger: Arc<Logger>` field to the artifact git backend struct in `src/artifact/git.rs`
- [x] 4.2 Update the artifact git backend constructor to accept `Arc<Logger>`
- [x] 4.3 Replace `eprintln!`("warning: artifact store sync failed...")` with `self.logger.warn("artifact store sync failed, reading stale local data")`

## 5. Main

- [x] 5.1 In `main.rs`, after loading config, call `config.logger()` and wrap in `Arc`
- [x] 5.2 Pass `Arc<Logger>` into `TrelloBackend::new`
- [x] 5.3 Pass `Arc<Logger>` into the artifact git backend constructor (via `build_artifact_store` or equivalent)
- [x] 5.4 Update `exit_error` to accept `&Logger` and call `logger.error(msg)` before `eprintln!` and `process::exit(1)`; update the single call site in `main`

## 6. Tests

- [x] 6.1 Unit test `Logger`: verify error/warn entries written to temp file with correct format
- [x] 6.2 Unit test `Logger`: verify debug entries suppressed when `debug = false`
- [x] 6.3 Unit test `Logger`: verify no panic when log file path is unwritable
- [x] 6.4 Update `TrelloBackend` tests that call `make_backend()` to supply a no-op logger (log to `/dev/null` or temp file)
- [x] 6.5 Run full test suite and confirm no regressions
