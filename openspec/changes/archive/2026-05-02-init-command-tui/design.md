## Context

`orga` currently has no setup path — users must hand-write `~/.orga/config.toml` with values that aren't easy to find (Trello board IDs and member IDs are not surfaced in the Trello UI). There is one backend (Trello) and one config schema. The `AppConfig::load()` function requires the file to exist and be valid before any command runs, so `init` must bypass it entirely.

## Goals / Non-Goals

**Goals:**
- Interactive step-by-step wizard using `inquire` for prompt rendering
- Mask the Trello token input
- Auto-fetch Trello member ID from `/1/members/me` after credentials are entered
- Auto-fetch and present a board picker from `/1/members/me/boards`
- Pre-populate prompts with values from any existing config
- Write the resulting TOML to the resolved config path (respects `--config` / `ORGA_CONFIG`)
- Create the config directory if it doesn't exist

**Non-Goals:**
- Supporting non-Trello backends in the wizard (backend is hardcoded to `trello` for now)
- Validating Trello credentials beyond what the API naturally returns (401 → clear error)
- Editing an existing config in-place with field-level diffs
- A `--non-interactive` / headless mode

## Decisions

### `inquire` as the TUI library

**Decision**: Add `inquire` as the sole new dependency.

**Alternatives considered**:
- `dialoguer`: similar weight, less ergonomic API for password fields and selection lists
- `ratatui`: full TUI framework — far too heavy for a sequential wizard
- `inquire` provides `Text`, `Password`, and `Select` prompt types that map directly to the three input modes needed, with built-in default-value support and validation hooks

### `init` bypasses `AppConfig::load()`

**Decision**: The `Commands::Init` arm in `main.rs` does not call `AppConfig::load()`. It calls a new `AppConfig::try_load()` that returns `Option<AppConfig>` (returns `None` if the file is absent or unparseable).

**Rationale**: The whole point of `init` is to create the config. Requiring a valid config to run any command is the current invariant for all other commands — `init` is the explicit exception.

### Trello API calls in `src/init.rs`, not via `TrelloBackend`

**Decision**: `init.rs` uses `reqwest::blocking::Client` directly to call two endpoints: `GET /1/members/me` and `GET /1/members/me/boards`. It does not construct a `TrelloBackend`.

**Rationale**: `TrelloBackend` requires a board ID to construct, which isn't known until the wizard completes. Extracting low-level HTTP helpers would add unnecessary abstraction. The two calls in `init` are simple enough to inline.

### TOML written as a format string

**Decision**: The final config is written using `format!()` to produce the TOML string rather than adding `Serialize` derives to all config structs.

**Rationale**: The config schema is small and stable. Adding `Serialize` to every struct solely for `init` would touch more code than necessary and could alter serialisation behaviour for other uses. A format string makes the exact output explicit and easy to audit.

### Board selection cursor starts at existing board if config present

**Decision**: When an existing config is loaded and its `board.id` matches a fetched board, the `inquire::Select` starting index is set to that board's position in the list.

**Rationale**: Re-running `init` to update a single credential (e.g., a rotated token) should not require re-selecting the board from scratch.

## Risks / Trade-offs

- **Trello API call fails mid-wizard** → The wizard aborts with a clear error message including the HTTP status. The user must re-run `init`. There is no partial-save or resume. Acceptable given the wizard is short (< 30 seconds to complete).
- **Member ID silently wrong** → If `/1/members/me` returns a different member than expected (unlikely but possible with shared API keys), the user has no prompt to catch it. Mitigated by printing `Authenticated as @<username> (<full_name>)` before proceeding.
- **`inquire` does not support Windows console well in some terminal emulators** → Out of scope; `orga` targets macOS/Linux agent environments.
- **TOML format string drift** → If new required config fields are added later, the format string in `init.rs` must be updated manually. Mitigated by a compile-time check: after writing the file, `init` calls `AppConfig::load()` on the result and returns an error if it fails to parse.

## Open Questions

- Should the memory path be prompted, or always use the default `~/.orga/memory.db`? → Defaulting silently is fine for now; power users can edit the file.
