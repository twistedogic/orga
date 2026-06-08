## 1. ContextRepository::delete

- [x] 1.1 Add `extract_significant_terms(description: &str) -> Vec<String>` helper in `src/memory.rs` — split on whitespace/punctuation, lowercase, keep words ≥ 3 chars, exclude stopwords (`the`, `and`, `for`, `not`, `are`, `was`, `but`, `its`)
- [x] 1.2 Implement `ContextRepository::delete(rel_path: &str) -> Result<(), OrgaError>` — reads file, extracts terms from frontmatter description, searches other `.md` files for any matching term, blocks if none found (unless description is absent/empty), deletes file, commits `delete: {path}`
- [x] 1.3 Add unit tests for `extract_significant_terms` — stopword filtering, punctuation stripping, short word removal
- [x] 1.4 Add unit tests for `ContextRepository::delete` — covered terms allowed, unique terms blocked, no frontmatter allowed, empty description allowed, non-existent file errors

## 2. memory_delete Tool

- [x] 2.1 Add `memory_delete` handler to `dispatch_sleep_tool` in `src/agent/tools.rs` — calls `ctx.context_repo.delete(path)`, returns success or error string
- [x] 2.2 Add `memory_delete` tool definition to a new `defrag_tool_definitions()` function in `src/agent/tools.rs` — NOT included in `all_tool_definitions()`
- [x] 2.3 Update `run_defrag_agent` in `src/agent/mod.rs` to use `defrag_tool_definitions()` instead of a manually constructed list

## 3. Defrag Agent Prompt

- [x] 3.1 Update `run_defrag_agent` system prompt in `src/agent/mod.rs` — remove hierarchy reorganization instructions; add explicit instructions to use `memory_delete` after merging duplicates; clarify that folder structure should not be changed

## 4. orga memory defrag CLI

- [x] 4.1 Add `DefragReport` struct in `src/memory.rs` with fields: `oversized: Vec<OversizedFile>`, `duplicates: Vec<DuplicatePair>`, `deletion_candidates: Vec<DeletionCandidate>` — all `serde::Serialize`
- [x] 4.2 Implement `ContextRepository::analyze() -> Result<DefragReport, OrgaError>` — walks all `.md` files, counts lines for oversized (>200), compares description terms between all pairs for duplicates (≥2 shared terms), checks deletion candidates (all terms covered elsewhere)
- [x] 4.3 Implement `orga memory defrag` command in `src/main.rs` — opens `ContextRepository`, calls `analyze()`, prints human-readable report or `--json` output; exits 0 with no output when report is empty

## 5. Tests

- [x] 5.1 Unit test `ContextRepository::analyze()` — oversized detection, duplicate pair detection, deletion candidate detection, empty repository
- [x] 5.2 Unit test `dispatch_sleep_tool("memory_delete", ...)` — successful delete, blocked delete returns error string
- [x] 5.3 Verify `memory_delete` is absent from `all_tool_definitions()`
