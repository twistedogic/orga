## 1. Dependencies & Config

- [x] 1.1 Add `git2` dependency to `Cargo.toml`
- [x] 1.2 Add `[memory]` config section to `AppConfig` with `path`, `defrag_file_threshold` (default: 20), and `defrag_size_threshold_kb` (default: 50) fields in `src/config.rs`
- [x] 1.3 Update `config.memory_db_path()` to return the new directory path (`~/.orga/memory/`) and add a helper `config.memory_repo_path()` for the git repo

## 2. ContextRepository

- [x] 2.1 Create `ContextRepository` struct in `src/memory.rs` with `open(path)` — initializes the git repo on first use, creates `system/` dir and initial `system/overview.md` stub if missing
- [x] 2.2 Implement `ContextRepository::list()` — walks all `.md` files, extracts frontmatter `description`, returns `Vec<(path, description)>`
- [x] 2.3 Implement `ContextRepository::read(path)` — returns file content or error if not found
- [x] 2.4 Implement `ContextRepository::write(path, content, commit_msg)` — writes file, auto-creates parent dirs, commits via `git2` with agent name as author
- [x] 2.5 Implement `ContextRepository::search(query)` — case-insensitive literal grep across all `.md` files, returns `Vec<(path, line_no, line)>`
- [x] 2.6 Implement `ContextRepository::repo_stats()` — returns file count and total size in KB (used by sleep-time threshold check)

## 3. Agent System Prompt Injection

- [x] 3.1 Update `build_system_prompt` in `src/agent/context.rs` to accept `ContextRepository` and inject the file tree index under `## Context Repository`
- [x] 3.2 Inject `system/` file contents fully under `## Context Repository (pinned)` in `build_system_prompt`
- [x] 3.3 Remove the `memory_store.get(&ticket.summary.id)` call from `build_user_message` — per-ticket memory no longer injected
- [x] 3.4 Update `build_subagent_context` to also inject the file tree index and `system/` contents

## 4. Memory Tools

- [x] 4.1 Add `memory_list`, `memory_read`, `memory_write`, `memory_search` tool definitions to `all_tool_definitions()` in `src/agent/tools.rs`
- [x] 4.2 Implement `dispatch_memory_list`, `dispatch_memory_read`, `dispatch_memory_write`, `dispatch_memory_search` handlers in `src/agent/tools.rs`
- [x] 4.3 Add `context_repo: ContextRepository` to `ToolContext` struct; thread it through `process_ticket` and `process_subagent_ticket` in `src/agent/mod.rs`
- [x] 4.4 Remove `set_memory` from the default tool set and all tool definitions
- [x] 4.5 Ensure memory tools are always included for both main agent and subagent tool sets (alongside `todos`)
- [x] 4.6 Update the narrowed main-agent tool set (subagent mode) to include the four memory tools instead of `set_memory`

## 5. Sleep-time Agent

- [x] 5.1 Add `run_sleep_time_agent` async function in `src/agent/mod.rs` — accepts completed ticket context + `ContextRepository` + LLM client, builds a reflection prompt, runs a short LLM loop with only `memory_list`, `memory_read`, `memory_write` tools
- [x] 5.2 Spawn `run_sleep_time_agent` after successful `done()` resolution in the ticket loop; log errors but do not fail the cycle
- [x] 5.3 After the reflection writes, check `repo_stats()` against thresholds; if exceeded, run `run_defrag_agent`
- [x] 5.4 Add `run_defrag_agent` async function — builds a defrag prompt, runs an LLM loop over the full repository, commits result as "defrag: reorganize context repository"

## 6. CLI Commands

- [x] 6.1 Replace `orga memory get` with `orga memory read <path>` in `src/main.rs` clap command structure
- [x] 6.2 Replace `orga memory set` with `orga memory write <path> <content>` (with optional `--message`) in `src/main.rs`
- [x] 6.3 Add `orga memory list` subcommand (with `--json`)
- [x] 6.4 Add `orga memory search <query>` subcommand (with `--json`)
- [x] 6.5 Add `orga memory defrag` subcommand that triggers a defrag pass manually
- [x] 6.6 Wire all new `orga memory` subcommands to `ContextRepository` methods

## 7. Remove Old MemoryStore

- [x] 7.1 Remove `MemoryStore` struct and `set`/`get` methods from `src/memory.rs`
- [x] 7.2 Remove all `MemoryStore` references from `src/agent/mod.rs`, `src/agent/tools.rs`, `src/agent/context.rs`
- [x] 7.3 Remove `dispatch_set_memory` handler and `SetMemoryArgs` from `src/agent/tools.rs`
- [x] 7.4 Verify `CompactionStore` and `TodoStore` (ticket-scoped) remain unaffected

## 8. Skill Update

- [x] 8.1 Update `skills/orga/SKILL.md` — replace "Load memory first" / "Save findings" sections (per-ticket `orga memory get/set`) with topic-based memory workflow: scan tree with `orga memory list`, read relevant files with `orga memory read`, write cross-ticket learnings with `orga memory write`
- [x] 8.2 Update the command reference table in `skills/orga/SKILL.md` to list new `orga memory` subcommands and remove old ones
- [x] 8.3 Add a note about the `system/` convention and when to write vs. when to let the sleep-time agent handle it

## 9. Tests

- [x] 9.1 Add unit tests for `ContextRepository::list()` — frontmatter extraction, empty repo, missing description
- [x] 9.2 Add unit tests for `ContextRepository::write()` — creates file, creates parent dirs, produces git commit
- [x] 9.3 Add unit tests for `ContextRepository::search()` — matching, no match, case-insensitive
- [x] 9.4 Update or remove existing `MemoryStore` tests
- [x] 9.5 Add integration test for sleep-time agent trigger after `done()` in dry-run mode (verify it is invoked, not that LLM writes occur)
