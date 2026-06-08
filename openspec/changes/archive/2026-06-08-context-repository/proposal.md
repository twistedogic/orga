## Why

Agent memory in orga is scoped per-ticket — every session starts without knowledge of prior work on other tickets. This means agents re-discover the same team conventions, recurring patterns, and architectural decisions over and over. Cross-ticket learning is impossible. Moving to a topic-organized, git-backed context repository makes agent memory cumulative and cross-cutting, matching how humans actually maintain institutional knowledge.

## What Changes

- **BREAKING** Replace per-ticket `MemoryStore` (SQLite) with a git-backed context repository on the local filesystem
- **BREAKING** Replace `orga memory get/set <ticket_id>` CLI commands with `orga memory list`, `orga memory read <path>`, `orga memory write <path> <content>`, and `orga memory search <query>`
- Add a `system/` directory convention — files in `system/` are always injected fully into agent context
- Inject context repository file tree index (with frontmatter descriptions) into every agent system prompt
- Add four new agent tools: `memory_list`, `memory_read`, `memory_write`, `memory_search` — exposed to all main agents and subagents by default
- Add a sleep-time agent that runs after `done()` — reflects on the completed ticket and writes learnings into topic files
- Add defragmentation logic inside the sleep-time agent, triggered when the repository crosses a size threshold (default: 20 files or 50KB total)
- **BREAKING** Remove `set_memory` tool from the agent tool set
- Update `skills/orga/SKILL.md` to document topic-based memory workflow

## Capabilities

### New Capabilities

- `context-repository`: Git-backed filesystem memory repository with topic-organized markdown files, frontmatter navigation index, `system/` always-loaded pinned files, and progressive disclosure via `memory_read`
- `memory-tools`: Four agent tools (`memory_list`, `memory_read`, `memory_write`, `memory_search`) exposed by default to all main agents and subagents
- `sleep-time-agent`: Background agent that runs after `done()`, reflects on completed ticket work, and persists learnings into the context repository; includes defragmentation pass when threshold is crossed

### Modified Capabilities

- `agent-memory`: Requirement changes from per-ticket SQLite blobs to topic-organized git-backed files; the memory contract (what gets stored, how it's accessed) changes fundamentally
- `cli-commands`: `orga memory` subcommands change from `get/set <ticket_id>` to `list/read/write/search` against topic paths
- `agent-tools`: Default tool set gains `memory_list`, `memory_read`, `memory_write`, `memory_search`; loses `set_memory`

## Impact

- `src/memory.rs` — `MemoryStore` replaced with `ContextRepository` struct backed by a local git repo
- `src/agent/tools.rs` — remove `set_memory`, add `memory_list/read/write/search`; add `dispatch` entries for sleep-time agent invocation
- `src/agent/context.rs` — `build_system_prompt` injects file tree index + `system/` contents; `build_user_message` no longer pulls from SQLite memory
- `src/agent/mod.rs` — after `done()` resolves, trigger sleep-time agent as async task
- `src/main.rs` — rework `orga memory` subcommands
- `src/config.rs` — add `[memory]` config section with `path` (default: `~/.orga/memory`) and `defrag_threshold` fields
- `skills/orga/SKILL.md` — update memory section to document topic-based workflow
- New dependency: `git2` crate for git operations on the context repository
