## Context

`orga init` today writes config via hand-formatted TOML strings in `write_config_file` / `write_linear_config_file`. Each function owns its own serialization logic and overwrites the entire file. This works for an initial write but makes partial updates (e.g. adding `[llm]` without disturbing `[trello]`) impossible without re-prompting everything.

The agent loop requires `[llm]` to run. There is currently no supported way to set it up interactively.

## Goals / Non-Goals

**Goals**
- `orga init agent` sets up `[llm]` and optional sections, merging into an existing config
- `AppConfig` is fully round-trippable (deserialize → modify → serialize → write) via `toml::to_string`
- The board init path (`orga init board`) preserves its existing behavior and UX exactly
- No new external dependencies

**Non-Goals**
- LLM key validation (no cheap test-call exists for all providers; deferred to first `orga agent` run)
- Subagent or workflow setup via `init agent`
- Config migration tooling

## Decisions

### 1. Serialize via `AppConfig` struct (not string patching)

Add `Serialize` to all config structs and use `toml::to_string(&config)` for all writes. This means `init agent` can: load the existing config (if any), mutate only the `llm`, `memory`, `workspace`, and `skills` fields, and write the whole struct back.

**Alternative considered**: regex/section patching on the raw TOML string. Rejected — fragile, doesn't handle comments or ordering edge cases, and requires maintaining two code paths.

### 2. `skip_serializing_if` on all `Option` and `Vec` fields

`toml 0.8` serializes `None` as nothing by default, but only when annotated with `skip_serializing_if = "Option::is_none"`. Without it, serialization errors at runtime. All `Vec` fields use `skip_serializing_if = "Vec::is_empty"` to avoid empty array noise.

### 3. `AppConfig::save(path)` as the single write entry point

Both board init paths and the new agent init path call `config.save(path)`. This deletes ~60 lines of duplicated format-string TOML in `init.rs` and makes future config additions a one-line change.

### 4. `Commands::Init` becomes a subcommand group

`Init` changes from a unit variant to `Init(InitCommands)` where `InitCommands` has `Board` and `Agent` variants. The dispatch in `main.rs` is updated accordingly. `orga init` alone shows clap's auto-generated subcommand help.

### 5. Provider selection drives model default

`init agent` presents `[anthropic, openai]` via `Select`. Based on selection, the model prompt defaults to `claude-opus-4-5` (Anthropic) or `gpt-4o` (OpenAI). The user can overwrite.

## Risks / Trade-offs

- **Field ordering may change**: `toml::to_string` serializes fields in struct declaration order, not original file order. Existing configs rewritten via `init board` will have deterministic but potentially reordered sections. → Acceptable; config is machine-written.
- **Comments stripped**: Any hand-written comments in the config file will be lost after re-running `init board`. → This was already true with the old string-format approach.
- **`skip_serializing_if` on `Vec`**: `workflow` and `subagents` are `Vec` fields with `#[serde(default)]`. If empty, they will be omitted from the written config — correct behavior, but worth noting that the attribute must be added alongside the existing `default`.

## Migration Plan

No migration needed. Existing config files continue to load unchanged. The only breaking change is the CLI surface (`orga init` → `orga init board`), which affects no config files or stored state.

Users running `orga init` will see:
```
error: 'orga init' requires a subcommand
  orga init board   Interactive setup wizard for board credentials
  orga init agent   Interactive setup wizard for agent (LLM) mode
```

## Open Questions

- None. All decisions are resolved.
