## Context

The native agent loop (`src/agent/`) currently builds its system prompt from two sources: a hardcoded agent identity paragraph and optional per-column workflow instructions from `[[workflow]]` config entries. There is no mechanism for injecting domain knowledge or behavioral guidance beyond these.

The agentskills.io specification defines a standard format for agent skills: a directory containing a `SKILL.md` file with YAML frontmatter (`name`, `description`, optional `metadata`) followed by Markdown instructions. This change adds a skill layer on top of the existing prompt construction pipeline.

## Goals / Non-Goals

**Goals:**
- Scan a configurable skills folder at ticket-load time
- List all discovered skills (name + description) in every system prompt
- Inject the full body of matched skills into the system prompt
- Support four matching signals: `orga-match-always`, `orga-match-column`, `orga-match-label` (in skill frontmatter), and `skill:<name>` ticket labels (human override)
- Log a warning when a `skill:` label references a skill not found in the folder
- Concatenate multiple matched skills in the order they are discovered

**Non-Goals:**
- Progressive disclosure / on-demand loading of skill reference files during the LLM loop
- Skills as callable tools (option 3)
- Validating skill content beyond frontmatter parsing
- Fetching skills from remote sources

## Decisions

### Frontmatter parsing without an external crate

The agentskills.io spec uses standard YAML frontmatter (delimited by `---`). Rather than pulling in a YAML crate, we parse only the fields we need (`name`, `description`, `metadata`) via simple string splitting. The `metadata` block contains flat key-value pairs which we parse line-by-line.

*Alternative considered*: `serde_yaml` — adds a dependency for a narrow use case. The frontmatter structure is simple enough that manual parsing is safer and dependency-free.

### Matching is OR across all signals

A skill activates if *any* matching signal fires: `orga-match-always`, `orga-match-column` (case-insensitive, same convention as `workflow_prompt`), `orga-match-label` (case-insensitive), or a `skill:<name>` ticket label. This is the most flexible model — skills can be globally applicable, column-scoped, label-scoped, or purely on-demand.

### `skill:` labels as human override

Ticket labels with the `skill:` prefix let humans explicitly request a skill for any ticket, regardless of the skill's own matching config. A skill with no `orga-match-*` metadata is valid and acts as a pure on-demand skill. Missing skills referenced via `skill:` label produce a `logger.warn` call — no crash, no prompt modification.

### System prompt injection (not user message)

Skills are behavioral guidance ("how to approach this type of work"), not data about the ticket. They belong in the system prompt alongside workflow instructions, not in the user message alongside ticket content.

Structure:
```
[agent identity]

## Available Skills
- **<name>**: <description>
...

## Column Instructions        ← existing, unchanged
<workflow prompt>

## Active Skills              ← new, only if skills matched
### <name>
<SKILL.md body>
```

### New module: `src/agent/skills.rs`

Skill scanning and matching is isolated in a new module. `build_system_prompt` in `context.rs` accepts a `SkillContext` struct (available skills list + active skill bodies) produced by `skills.rs`. This keeps context.rs focused on prompt assembly and makes the skill logic independently testable.

### `[skills]` config is optional

No `[skills]` section = skills feature disabled. Existing configs continue to work without modification.

## Risks / Trade-offs

- **Large skill bodies bloat the system prompt** → Mitigated by the agentskills.io recommendation to keep `SKILL.md` under 500 lines; operators are responsible for skill size. We do not enforce a cap.
- **Frontmatter parse errors silently skip a skill** → We log a warning and continue rather than failing the whole ticket cycle. A malformed skill should not block the agent.
- **Column/label matching is case-insensitive string equality** → Simple and predictable, but won't handle partial matches or wildcards. Can be extended later.
- **Ticket label structure is backend-specific** → Trello and Linear both support labels; the `Ticket` model needs to expose them. If a backend doesn't populate labels, `skill:` hints simply won't fire.

## Migration Plan

No migration needed. The `[skills]` config section is optional; existing deployments are unaffected. Operators adopt by adding the section and populating a skills folder.

## Open Questions

- Does the `Ticket` / `TicketSummary` model currently expose labels? If not, that's a prerequisite. *(To verify during implementation.)*
