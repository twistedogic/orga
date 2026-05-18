## Context

orga is invoked as a subprocess by LLM agents. Currently, when Trello returns a non-2xx HTTP response (including the non-standard 449 "Retry With"), orga discards the response body and only surfaces the status code in the error message. There is no persistent log, so the agent has no way to diagnose API failures after the fact. A non-fatal git sync warning in the artifact backend also uses bare `eprintln!` with no timestamp and no persistence.

The current error surface:
- `src/main.rs:exit_error` — `eprintln!("error: {msg}")` + `process::exit(1)`
- `src/board/trello.rs:check_status` — borrows `&Response`, drops body on error, returns only status code
- `src/artifact/git.rs:283` — `eprintln!("warning: artifact store sync failed...")` with no context

No logging crates are currently used. The project keeps dependencies minimal.

## Goals / Non-Goals

**Goals:**
- Capture all 4xx/5xx HTTP response bodies from Trello and write them to a log file with timestamp + level
- Write fatal errors and non-fatal warnings to the same log file
- Support optional debug-level logging (toggled via config)
- Default log path (`~/.orga/orga.log`) so logging works without any config change
- No new crate dependencies

**Non-Goals:**
- Structured/JSON log output
- Log rotation or size limits
- Logging to stdout/stderr in addition to file (stderr behavior unchanged)
- Tracing integration or spans
- Per-command log context (no request IDs)

## Decisions

### Decision: No logging framework — use `std::fs` directly

**Chosen**: Simple `Logger` struct wrapping `Mutex<Option<File>>` opened in append mode.

**Alternatives considered**:
- `tracing` + `tracing-appender`: industry standard, but adds ~8 dependencies; excessive for this use case
- `log` + `simplelog`: lighter, ~3 deps; still more than needed given only two log sites today
- `env_logger`: stdout/stderr only, no file support without extra work

**Rationale**: The project has no logging today; there are 3 log sites total. A 30-line custom struct is the right size. If the project grows to need structured logs or spans, migrating to `tracing` is straightforward since `Logger` will be threaded through all the right places already.

### Decision: `check_status` becomes `handle_response` — consumes the response

**Chosen**: Change signature from `check_status(&self, resp: &Response) -> Result<(), OrgaError>` to `handle_response(&self, resp: Response) -> Result<String, OrgaError>`. On success returns the body as a `String`; callers use `serde_json::from_str(&body)` instead of `resp.json()`.

**Alternatives considered**:
- Read body bytes and re-wrap into a new response: not possible with reqwest's blocking client
- Keep `check_status` as-is, read body separately before calling it: would require calling `.text()` unconditionally on every response, making the happy path also pay for a string allocation

**Rationale**: Consuming the response is the only clean way to access the body in reqwest's blocking API. The double-parse cost (text → string → serde_json) is negligible for this workload. This also simplifies callers: one method handles both status check and body extraction.

### Decision: `Arc<Logger>` threaded through backends

**Chosen**: `Logger` is constructed in `main`, wrapped in `Arc`, and passed into `TrelloBackend::new` and the artifact git backend constructor.

**Alternatives considered**:
- Global/lazy_static logger: avoids threading it through constructors, but introduces global mutable state and makes testing harder
- Thread-local storage: same problems, worse ergonomics

**Rationale**: `Arc<Logger>` is idiomatic, testable, and consistent with how other dependencies (`AppConfig`) flow through the codebase.

### Decision: Log file always on by default

**Chosen**: If `[logging]` section is absent from config, use `~/.orga/orga.log` as the default path. `debug` defaults to `false`.

**Alternatives considered**:
- Only log if `[logging]` section is present: cleaner config contract, but means newly deployed agents get no log until someone explicitly configures it

**Rationale**: Logs are most valuable when something goes wrong unexpectedly. Requiring opt-in means the log won't exist the first time an agent hits 449. Default-on with a predictable path is better for the agent-facing use case.

## Risks / Trade-offs

- **[Risk] File write failures silently ignored** → If the log file can't be written (permissions, disk full), logging fails silently. The operation proceeds normally. This is intentional — a logging failure should never cause a command to fail.
- **[Risk] Log file grows unbounded** → No rotation. For the expected invocation frequency (agent skill, not daemon), this is acceptable. Can be addressed later with log rotation support.
- **[Risk] `handle_response` buffers entire body in memory** → Trello API responses are small JSON objects. Not a concern for this workload.

## Open Questions

None. Design is complete.
