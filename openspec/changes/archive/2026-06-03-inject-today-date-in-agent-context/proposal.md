## Why

Agents operating on tickets have no awareness of the current date, which limits their ability to reason about deadlines, schedules, or time-sensitive context. Injecting today's date into every agent context resolves this without requiring agents to fetch it themselves.

## What Changes

- `build_user_message()` in `src/agent/context.rs` gains a `**Today's date:** YYYY-MM-DD` field in the ticket metadata header
- This single change covers both main agent and subagent, since both call `build_user_message()` internally

## Capabilities

### New Capabilities

- `agent-date-context`: Today's date is injected into every agent context (main and subagent) as part of the user message metadata

### Modified Capabilities

- `agent-loop`: The user message now includes a `Today's date` field alongside existing ticket metadata

## Impact

- `src/agent/context.rs`: one line added to `build_user_message()`
- `chrono` crate already present in `Cargo.toml` — no new dependencies
- No changes to tool definitions, config schema, or external interfaces
