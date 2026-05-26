## 1. Ticket Model: Labels

- [x] 1.1 Add `labels: Vec<String>` field to `TicketSummary` in `src/models.rs`
- [x] 1.2 Populate `labels` from Trello card label names in `src/board/trello.rs`
- [x] 1.3 Populate `labels` from Linear issue labels in `src/board/linear.rs` (if Linear backend exists)
- [x] 1.4 Update `TicketSummary` construction in tests to include empty `labels` vec

## 2. Config: Skills Section

- [x] 2.1 Add `SkillsConfig` struct with `path: String` field to `src/config.rs`
- [x] 2.2 Add `pub skills: Option<SkillsConfig>` to `AppConfig`
- [x] 2.3 Add `skills_path()` helper on `AppConfig` that returns `Option<PathBuf>` (tilde-expanded)
- [x] 2.4 Add config tests: skills section present, skills section absent, path tilde expansion

## 3. Skills Module: Scanning and Matching

- [x] 3.1 Create `src/agent/skills.rs` with `SkillMeta` struct (`name`, `description`, `body`, `match_always`, `match_column`, `match_label`)
- [x] 3.2 Implement frontmatter parser: split on `---`, extract `name`, `description`, and `metadata` key-value pairs; return `None` with a warning on parse failure
- [x] 3.3 Implement `scan_skills(path, logger) -> Vec<SkillMeta>`: walk subdirs, parse each `SKILL.md`, skip invalid with warning, warn if folder missing
- [x] 3.4 Implement `match_skills(skills, ticket, logger) -> Vec<&SkillMeta>`: OR across all four signals; deduplicate; warn on missing `skill:` label references
- [x] 3.5 Add `pub mod skills;` to `src/agent/mod.rs`
- [x] 3.6 Unit tests for frontmatter parsing: valid, missing name, malformed delimiter
- [x] 3.7 Unit tests for matching: orga-match-always, orga-match-column (case-insensitive), orga-match-label, skill: label, missing skill: label (warning), deduplication, multiple matches

## 4. System Prompt: Skills Injection

- [x] 4.1 Add `SkillContext` struct to `src/agent/context.rs` with `available: Vec<(String, String)>` and `active: Vec<(String, String)>` (name, body)
- [x] 4.2 Update `build_system_prompt` signature to accept `Option<&SkillContext>`
- [x] 4.3 Append "## Available Skills" section to system prompt when `skill_ctx` has any available skills
- [x] 4.4 Append "## Active Skills" section with `### <name>` + body for each active skill
- [x] 4.5 Update `build_context` signature to accept `Option<&SkillContext>` and pass it through to `build_system_prompt`
- [x] 4.6 Update context tests: system prompt includes available skills, active skills injected, no section when no skills

## 5. Agent Loop: Wire Skills

- [x] 5.1 In `process_ticket` (`src/agent/mod.rs`), call `scan_skills` using `config.skills_path()` before building context
- [x] 5.2 Call `match_skills` against the ticket to produce active skills list
- [x] 5.3 Construct `SkillContext` and pass to `build_context`
- [x] 5.4 Integration test or manual verification: agent processes ticket with a skills folder configured and active skill appears in system prompt
