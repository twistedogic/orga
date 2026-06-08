## Why

The defrag agent currently has no way to delete files, so merging duplicates leaves orphaned originals that grow the repository indefinitely. The `orga memory defrag` CLI command is also a stub that does nothing. The defrag scope is also too broad — reorganizing folder hierarchy is too opinionated for a background agent.

## What Changes

- Add `ContextRepository::delete(path)` with a guardrail: deletion is blocked if the file's frontmatter `description` keywords appear in no other file; files with no frontmatter are always deletable
- Add `memory_delete` tool to the defrag agent and `SleepToolContext`; the tool is **not** exposed to the main agent or subagents
- Narrow the defrag agent prompt to cleanup only: split oversized files, merge + delete duplicates — hierarchy reorganization removed
- Implement `orga memory defrag` CLI command as a real analysis report: lists oversized files, likely duplicates (by shared description terms), and deletion candidates; no mutations; supports `--json`

## Capabilities

### New Capabilities

- `memory-delete`: `ContextRepository::delete()` with frontmatter-based uniqueness guardrail, `memory_delete` tool scoped to defrag agent, and commit on successful deletion

### Modified Capabilities

- `sleep-time-agent`: Defrag agent scope narrowed to cleanup only (split, merge+delete); `memory_delete` added to defrag tool set; hierarchy reorganization removed from prompt
- `cli-commands`: `orga memory defrag` implemented as analysis report (was a stub) with human-readable and `--json` output modes

## Impact

- `src/memory.rs` — add `ContextRepository::delete()` with guardrail logic
- `src/agent/tools.rs` — add `memory_delete` tool definition and `dispatch_sleep_tool` handler; tool is defrag-only, not in `all_tool_definitions()`
- `src/agent/mod.rs` — update `run_defrag_agent` prompt and tool set; add `memory_delete` to defrag's `SleepToolContext` tool list
- `src/main.rs` — implement `orga memory defrag` subcommand as analysis report
