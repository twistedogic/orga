## Context

The context repository defrag agent currently has no delete capability. It can write new files but cannot remove originals when merging, leaving the repository growing unboundedly. The `orga memory defrag` CLI is a stub. This change adds deletion with a safety guardrail and implements the CLI as a real analysis report.

Current state:
- `run_defrag_agent` in `src/agent/mod.rs` has tools: `memory_list`, `memory_read`, `memory_write`
- `orga memory defrag` prints "not available from CLI" and exits
- Defrag prompt instructs the agent to reorganize hierarchy — too broad for a cleanup agent

## Goals / Non-Goals

**Goals:**
- `ContextRepository::delete(path)` — deletes a file and commits, blocked by frontmatter uniqueness guardrail
- `memory_delete` tool — available to defrag agent only (not main agent, not subagents)
- Defrag agent scope narrowed: split oversized files, merge + delete duplicates; no hierarchy reorganization
- `orga memory defrag` — real analysis report: oversized files, duplicate candidates, deletion candidates; `--json` supported

**Non-Goals:**
- Automatic deletion without agent involvement (the LLM still decides what to delete)
- Exposing `memory_delete` to main agent or subagents (cleanup is defrag-agent-only)
- Undo/recovery for deletions (git history is the recovery mechanism)
- Making the CLI defrag command invoke an LLM pass (report only, no mutations)

## Decisions

### D1: Guardrail derived from frontmatter `description`, not file content

**Decision:** When `delete(path)` is called, extract significant terms from the file's `description` frontmatter field (words ≥ 4 chars, lowercase, punctuation stripped). Search each term across all other `.md` files. If at least one term matches anywhere else, deletion is allowed. If no terms match, block with an informative error.

**Rationale:** Content-level deduplication is expensive (full-text comparison, LLM similarity) and fragile. Frontmatter descriptions are agent-authored summaries of what the file covers — using them is fast, deterministic, and aligns with the human-readable navigation contract already established for the repository.

**Files with no frontmatter:** Always allowed to delete. No description = no claimed unique content.

**Alternative considered:** Block deletion if any content line is unique across the repo. Rejected — expensive, and generic lines (headers, blanks) would trigger false blocks.

### D2: `memory_delete` is defrag-only — not in `all_tool_definitions()`

**Decision:** `memory_delete` is added to `dispatch_sleep_tool` in `SleepToolContext` but is NOT added to `all_tool_definitions()`. It is only passed to the defrag agent's tool set explicitly.

**Rationale:** Main agents and subagents working on tickets should not be able to delete memory files — deletion is a maintenance operation, not a ticket-cycle operation. Keeping it out of the global tool set prevents accidental or prompted deletion during ticket work.

### D3: `orga memory defrag` is analysis-only (no LLM, no mutations)

**Decision:** The CLI command performs all analysis locally using `ContextRepository` methods, outputs a structured report, and exits. It does not call an LLM or trigger the defrag agent.

**Rationale:** A mutation-free CLI command is safe to run at any time, easy to script, and gives humans visibility into repository health without risk. The LLM defrag pass remains agent-loop-only (triggered automatically after `done()` when thresholds are exceeded).

**Analysis performed:**
1. **Oversized files** — `.md` files over 200 lines
2. **Duplicate candidates** — pairs of files that share ≥ 2 significant description terms
3. **Deletion candidates** — files whose all description terms appear in at least one other file (i.e., would pass the delete guardrail)

### D4: Significant term extraction — words ≥ 4 chars

**Decision:** Split the frontmatter `description` on whitespace and punctuation, lowercase everything, keep words ≥ 4 characters, deduplicate.

**Rationale:** Short words (≥ 1-3 chars) are stopwords ("the", "and", "for", "is") that would produce false coverage matches everywhere. 4-character minimum filters these reliably without requiring a stopword list.

**Example:** `"Auth JWT investigation notes"` → `["auth", "investigation", "notes"]` (JWT is 3 chars, excluded)

Actually JWT is valuable — reconsider: keep words ≥ 3 chars, exclude a small hardcoded stopword set (`the`, `and`, `for`, `not`, `are`, `was`, `but`, `its`). This captures acronyms like JWT, API, SQL.

**Revised decision:** Terms ≥ 3 chars, excluding hardcoded stopwords.

## Risks / Trade-offs

- **[Risk] False "covered" result** — a term from the deleted file's description appears in another file but in an unrelated context (e.g., "auth" appears in a file about "authorizing deployments"). → Mitigation: the guardrail is best-effort safety, not semantic understanding. The defrag agent makes the final judgment; the guardrail just prevents accidental deletion of truly unique content.
- **[Risk] Defrag agent deletes too aggressively** — with `memory_delete` available, the agent may over-delete. → Mitigation: the guardrail blocks deletion of genuinely unique files; git history allows recovery.
- **[Risk] CLI analysis disagrees with agent behavior** — the report shows a file as "deletion candidate" but the agent decides not to delete it (or vice versa). → Acceptable: the report is a human-readable hint, not a contract.

## Migration Plan

No migration needed. Existing repositories gain `delete` capability transparently. The CLI command goes from stub to working analysis.

## Open Questions

None — all decisions resolved during exploration.
