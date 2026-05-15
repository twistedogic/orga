## Why

Agents have no way to know what behaviour is expected of them for a given board column. A ticket in "To Do" might call for exploration, while one in "Review" requires a different stance. Without a mapping, the agent must infer intent from ticket content alone, which is unreliable.

## What Changes

- New optional `[[workflow]]` config section in `config.toml` mapping column names to prompt text
- Each entry specifies a column name and either an inline `prompt` string or a `prompt_file` path
- `orga ticket show` output gains an optional `workflow_prompt` field (JSON) / `## Workflow` block (text) when the ticket's column has a mapping
- Config validation at load time: hard-fail if `prompt_file` does not exist or is unreadable; hard-fail if an entry has neither or both of `prompt`/`prompt_file`
- Column name matching is case-insensitive

## Capabilities

### New Capabilities

- `workflow-column-mapping`: Per-column workflow prompt injection — config-driven mapping from column name to prompt text, resolved and validated at startup, surfaced in `ticket show` output

### Modified Capabilities

- `config`: New `[[workflow]]` array section with validation rules
- `cli-commands`: `ticket show` output gains optional workflow prompt field

## Impact

- `src/config.rs` — new `WorkflowEntry` struct, `workflow` field on `AppConfig`, validation logic, prompt resolution method
- `src/main.rs` — `TicketCommands::Show` reads resolved prompt and injects into output
- `~/.orga/config.toml` — new optional section; no migration needed for existing configs
