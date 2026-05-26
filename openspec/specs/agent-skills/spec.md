## ADDED Requirements

### Requirement: Skills folder scanning
The agent loop SHALL scan the configured skills folder at ticket-load time. Each subdirectory containing a `SKILL.md` file SHALL be treated as a skill. Subdirectories without `SKILL.md` SHALL be silently ignored.

#### Scenario: Valid skill directory discovered
- **WHEN** the skills folder contains a subdirectory with a `SKILL.md` file
- **THEN** the skill is loaded and its `name` and `description` are available for prompt injection

#### Scenario: Subdirectory without SKILL.md is ignored
- **WHEN** a subdirectory exists in the skills folder but contains no `SKILL.md`
- **THEN** it is silently skipped and does not appear in the skills list

#### Scenario: Skills folder does not exist
- **WHEN** the configured skills folder path does not exist on disk
- **THEN** the agent logs a warning and proceeds with no skills loaded

### Requirement: SKILL.md frontmatter parsing
Each `SKILL.md` SHALL be parsed for YAML frontmatter delimited by `---`. The `name` and `description` fields SHALL be extracted. The `metadata` block SHALL be parsed for `orga-match-*` keys. A skill with unparseable frontmatter SHALL be skipped with a logged warning.

#### Scenario: Valid frontmatter parsed
- **WHEN** a `SKILL.md` contains valid frontmatter with `name` and `description`
- **THEN** both fields are extracted and the skill is available for matching

#### Scenario: Missing name field
- **WHEN** a `SKILL.md` has frontmatter but no `name` field
- **THEN** the skill is skipped with a warning logged

#### Scenario: Malformed frontmatter
- **WHEN** a `SKILL.md` has no opening `---` delimiter
- **THEN** the skill is skipped with a warning logged

### Requirement: Skill matching
A skill SHALL be activated for a ticket if any of the following signals fire:
- `orga-match-always: "true"` is set in the skill's `metadata`
- `orga-match-column: "<name>"` matches the ticket's column name (case-insensitive)
- `orga-match-label: "<label>"` matches any of the ticket's labels (case-insensitive)
- The ticket has a label with the prefix `skill:` followed by the skill's name (e.g., `skill:code-review`)

#### Scenario: orga-match-always activates skill on every ticket
- **WHEN** a skill has `orga-match-always: "true"` in its metadata
- **THEN** the skill is activated regardless of the ticket's column or labels

#### Scenario: orga-match-column activates skill for matching column
- **WHEN** a skill has `orga-match-column: "Review"` and the ticket is in the "Review" column
- **THEN** the skill is activated

#### Scenario: orga-match-column match is case-insensitive
- **WHEN** a skill has `orga-match-column: "review"` and the ticket column is "Review"
- **THEN** the skill is activated

#### Scenario: orga-match-label activates skill for matching ticket label
- **WHEN** a skill has `orga-match-label: "security"` and the ticket has a "security" label
- **THEN** the skill is activated

#### Scenario: skill: label activates named skill
- **WHEN** a ticket has the label `skill:code-review` and a skill named `code-review` exists
- **THEN** the `code-review` skill is activated regardless of its own matching metadata

#### Scenario: skill: label references missing skill
- **WHEN** a ticket has the label `skill:nonexistent` and no skill with that name exists
- **THEN** a warning is logged; no other action is taken

#### Scenario: Multiple signals fire for the same skill
- **WHEN** a skill matches via both `orga-match-column` and a `skill:` label
- **THEN** the skill is activated once (no duplication)

#### Scenario: Multiple skills match
- **WHEN** two or more skills match a ticket
- **THEN** all matched skills are activated; their bodies are concatenated in discovery order

### Requirement: Available skills listing in system prompt
Every system prompt SHALL include an "Available Skills" section listing the `name` and `description` of all discovered skills, regardless of whether any matched the current ticket.

#### Scenario: Available skills section always present when skills folder is configured
- **WHEN** the skills folder contains one or more valid skills
- **THEN** the system prompt includes an "## Available Skills" section with all skill names and descriptions

#### Scenario: No skills folder configured
- **WHEN** `[skills]` is absent from config
- **THEN** no "Available Skills" section appears in the system prompt

### Requirement: Active skills injection in system prompt
When one or more skills are activated for a ticket, their full `SKILL.md` body content (everything after the frontmatter) SHALL be injected into an "## Active Skills" section of the system prompt after the "## Available Skills" section. Each skill body SHALL be prefixed with its name as a level-3 heading.

#### Scenario: Active skills section injected for matched skills
- **WHEN** two skills match the current ticket
- **THEN** the system prompt contains "## Active Skills" with both skill bodies concatenated under their respective `### <name>` headings

#### Scenario: No active skills section when no skills match
- **WHEN** no skills match the current ticket
- **THEN** no "## Active Skills" section appears in the system prompt
