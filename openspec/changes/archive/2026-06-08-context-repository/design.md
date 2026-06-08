## Context

Currently, orga's `MemoryStore` is a SQLite table keyed by `ticket_id`. Agents use `set_memory(context)` to write free-form text and it is injected into the user message at the start of each ticket session. This works for single-ticket continuity but provides nothing across tickets — the agent rediscovers the same team conventions, architectural patterns, and recurring problems every time it works a new ticket.

The goal is a filesystem-based, git-backed context repository where memory is organized by topic rather than ticket. Files are navigable by the agent through a persistent tree index injected into every system prompt, with `system/` files always fully loaded and other files loaded on demand.

## Goals / Non-Goals

**Goals:**
- Replace per-ticket SQLite memory with a git-backed directory of topic markdown files
- Inject a lightweight file tree index (filenames + frontmatter descriptions) into every agent system prompt
- Always fully load `system/` files (pinned context — board overview, team conventions)
- Provide `memory_list`, `memory_read`, `memory_write`, `memory_search` as default tools for all agents and subagents
- Run a sleep-time agent after `done()` that reflects on the completed ticket and writes learnings to topic files
- Trigger a defragmentation pass inside the sleep-time agent when repository crosses a threshold (20 files or 50KB)
- Update `orga memory` CLI subcommands to match the new model
- Update `skills/orga/SKILL.md` to document the new topic-based memory workflow

**Non-Goals:**
- Multi-agent concurrent writes / git worktrees (single-agent architecture; not needed now)
- RAG / vector search over memory (grep-based search is sufficient)
- Migration of existing per-ticket SQLite memory entries (start fresh; old entries are ticket-scoped blobs with limited cross-ticket value)
- Remote git push of the memory repository
- Per-ticket memory retention alongside the new system

## Decisions

### D1: Filesystem over SQLite for memory storage

**Decision:** Replace `MemoryStore` SQLite with a plain filesystem directory initialized as a git repo.

**Rationale:** Files are universally accessible to both agents (via shell tools) and humans. Git provides a free audit trail with meaningful commit messages. Directory structure + filenames act as a navigational index that costs very little context. SQLite blobs are opaque, hard to inspect, and provide no structure for cross-topic recall.

**Alternative considered:** Keep SQLite, add a `topic` column. Rejected — still opaque to agents, no git history, no human-editable, frontmatter navigation impossible.

### D2: Frontmatter descriptions + file tree always in system prompt

**Decision:** The full file tree (path + frontmatter `description` field) is rendered into every agent system prompt. `system/` file contents are also always fully loaded. Other files are loaded on demand via `memory_read`.

**Rationale:** The agent needs to know what exists before it can decide what to read. A tree index of ~50 tokens is negligible. Full progressive disclosure (read everything upfront) would waste context on irrelevant material; full opacity (no index) means the agent can't navigate.

**Alternative considered:** Always load all memory files. Rejected — unbounded context growth as the repository grows.

### D3: Sleep-time agent triggered after `done()`

**Decision:** When `done()` resolves successfully, the agent loop spawns a sleep-time reflection pass before exiting the ticket cycle. The sleep-time agent receives the completed ticket context (description + comments) and the current memory file tree, and is prompted to persist learnings into topic files.

**Rationale:** `done()` is the natural consolidation point — the ticket work is complete and fresh in context. An async trigger means zero latency impact on the ticket cycle itself.

**Alternative considered:** Systemd timer for periodic reflection. Can be added later as a complement, but `done()` trigger is more targeted and sufficient as the primary mechanism.

### D4: Defragmentation as a threshold-triggered pass inside sleep-time agent

**Decision:** During each sleep-time reflection pass, after writing new learnings, the agent checks the repository size. If it exceeds 20 files or 50KB total, a defragmentation subagent is spawned. The defrag agent reads all files, splits oversized ones, merges duplicates, and rewrites the hierarchy.

**Rationale:** Defragmentation needs to happen automatically without user intervention. Attaching it to the sleep-time agent (which already has a full view of the repository) keeps the architecture simple. Threshold-based triggering avoids unnecessary LLM calls on every `done()`.

**Alternative considered:** Separate `orga memory defrag` CLI command (manual). Kept as an opt-in CLI command in addition to the automatic trigger.

### D5: `git2` crate for repository management

**Decision:** Use the `git2` crate (libgit2 bindings) to initialize, commit, and query the memory git repository.

**Rationale:** Pure Rust, well-maintained, no external git binary required. Each `memory_write` call produces a commit with the topic path and a summary message as the commit message.

**Alternative considered:** Shell out to `git` CLI. Rejected — fragile, requires git installed, harder to test.

### D6: `memory_search` uses grep over the repository

**Decision:** `memory_search(query)` runs a case-insensitive literal grep across all `.md` files in the context repository and returns matching lines with their file paths.

**Rationale:** Grep is fast, deterministic, and transparent. No embedding model or vector index needed for a repository capped at ~50 files. The agent can follow up with `memory_read` on relevant hits.

**Alternative considered:** Semantic/vector search. Overkill for this scale; adds infrastructure complexity.

## Risks / Trade-offs

- **Frontmatter quality degrades navigation** → The sleep-time agent must write accurate `description` frontmatter. Bad names/descriptions break discoverability. Mitigation: the sleep-time agent prompt explicitly instructs accurate frontmatter; defrag agent rewrites frontmatter during cleanup.
- **`system/` files grow unbounded** → If agents keep appending to `system/overview.md` it bloats the always-loaded context. Mitigation: defrag agent enforces size limits on `system/` files; convention is `system/` files stay under 200 lines.
- **Sleep-time agent fails silently** → If the reflection LLM call errors, the ticket cycle has already completed. Mitigation: log errors prominently; missing memory writes are recoverable (next `done()` will reflect on the ticket's comments).
- **git2 adds a native dependency** → libgit2 is a C library, which complicates cross-compilation. Mitigation: `git2` is already used in the artifact store (`WorkspaceStore`); this is consistent with existing practice.

## Migration Plan

1. The new `ContextRepository` is initialized on first use at the configured path (`~/.orga/memory/` by default). No migration of existing SQLite entries — start fresh.
2. The old `[memory] path` config key (`~/.orga/memory.db`) is repurposed to `[memory] path` pointing to the new directory. The `.db` default path is simply abandoned; users with custom paths should update their config.
3. `CompactionStore` and `TodoStore` remain in SQLite alongside the old memory DB path (they are ticket-scoped and unaffected).
4. After deploy, the first `done()` call seeds the repository with an initial reflection. Running `orga memory list` before that returns an empty tree.

## Open Questions

- Should subagents be allowed to call `memory_write` directly, or should memory writes be main-agent-only? Current decision: all agents (main + subagents) get all four memory tools for consistency, but subagents typically won't need to write.
- Should `system/` file list be configurable (pinning specific non-system/ files)? Deferred — the convention is sufficient for now.
