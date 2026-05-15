## Context

`orga` agents pick up tickets from a kanban board and work through them. Different columns represent different stages — "To Do", "In Progress", "Review" — each with different expectations of the agent. Currently the agent infers what to do from ticket content alone. The config system already handles per-deployment customisation (backends, memory paths, artifact stores); workflow mapping follows the same pattern.

The `AppConfig` struct in `src/config.rs` is loaded once at startup and validated before any command runs. The `ticket show` command in `src/main.rs` is the sole output point that needs enrichment.

## Goals / Non-Goals

**Goals:**
- Config-driven mapping from column name → prompt text, resolved at startup
- Support both inline text (`prompt`) and file-based prompts (`prompt_file`) per entry
- Hard-fail at load time on misconfigured or missing `prompt_file`
- Case-insensitive column name matching at lookup time
- Surface resolved prompt in `ticket show` output (JSON field + human text block)

**Non-Goals:**
- Workflow prompts in `ticket list` output — list is a scan surface, not a work surface
- Dynamic reload of prompt files without process restart
- Prompt templating or variable interpolation
- Per-agent or per-ticket workflow overrides

## Decisions

### Prompt storage: resolved at load vs. at lookup

**Decision**: Resolve `prompt_file` contents into memory at `AppConfig::load` time.

Alternatives considered:
- Lazy resolution at `ticket show` call time — defers errors, violates the "validate at startup" convention already established by backend and trello section validation
- Store path only, resolve later — same problem; a missing file would only surface during `show`, not `columns` or `whoami`

Resolving at load means the `WorkflowEntry` after validation always holds a ready `String`. No file I/O at command time.

### Config shape: array of tables vs. inline table map

**Decision**: `[[workflow]]` array of tables.

```toml
[[workflow]]
column = "To Do"
prompt = "Enter explore mode..."

[[workflow]]
column = "In Progress"
prompt_file = "~/.orga/prompts/in-progress.md"
```

Alternatives considered:
- `[workflow.columns]` flat map `"To Do" = "text"` — can't support `prompt_file` cleanly alongside inline text
- `[workflow]` with nested tables keyed by column name — awkward TOML for variable keys with structured values

Array of tables is idiomatic TOML for a list of structured entries and mirrors how other config sections (e.g., future multiple boards) would look.

### Matching: column name, not column ID

**Decision**: Match on `list_name` (human-readable), case-insensitive.

Trello column IDs are opaque strings; humans writing config files know column names. Case-insensitive folding (`to_lowercase` comparison) covers "To Do" vs "to do" vs "TO DO".

### Output injection: presentation layer only

**Decision**: `workflow_prompt` is not added to `Ticket` or `TicketSummary` structs. It is resolved in `main.rs` and injected into output at render time.

The data model represents board state. Workflow prompts are a local config concern — they should not leak into the shared data types used by the board backend or tests.

### Validation: exactly one of prompt/prompt_file

**Decision**: Each `[[workflow]]` entry must have exactly one of `prompt` or `prompt_file`. Having neither or both is a `ConfigError` at load time.

Serde deserialization loads both as `Option<String>`. Validation in `AppConfig::validate()` checks the invariant for each entry before the struct is returned to callers.

## Risks / Trade-offs

- **Config coupling to filesystem**: `prompt_file` paths are read at startup. If the file is on a remote mount that becomes unavailable, `orga` fails to start. Mitigation: document that `prompt_file` should be a local path; `~` expansion is supported.
- **Silent no-op on column name typo**: If the config spells "To Do" as "Todo", no error is raised — the prompt simply isn't injected. Mitigation: document matching is exact (case-insensitive) and add a note in the config reference. A future `orga workflow check` command could validate against live columns.
- **Prompt size in JSON output**: Large prompt files bloat `ticket show --json`. Acceptable for now given prompts are typically hundreds of bytes. No truncation logic added.
