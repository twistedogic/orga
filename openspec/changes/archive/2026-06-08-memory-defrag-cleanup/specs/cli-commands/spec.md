## MODIFIED Requirements

### Requirement: memory defrag subcommand
The CLI SHALL provide `orga memory defrag` as a read-only analysis report of the context repository. It SHALL NOT invoke an LLM or mutate any files. With `--json`, output SHALL be a structured JSON object.

The report SHALL include three sections:
1. **Oversized files** — `.md` files over 200 lines, listed with path and line count
2. **Duplicate candidates** — pairs of files sharing ≥ 2 significant description terms (terms ≥ 3 chars, excluding stopwords), listed with the shared terms
3. **Deletion candidates** — files where all description terms appear in at least one other file (i.e., would pass `memory_delete` guardrail), listed with which file covers them

#### Scenario: Report shows oversized files
- **WHEN** `orga memory defrag` is called and a file exceeds 200 lines
- **THEN** that file appears in the oversized section with its line count

#### Scenario: Report shows duplicate candidates
- **WHEN** two files share ≥ 2 significant description terms
- **THEN** they appear as a pair in the duplicate candidates section with the shared terms listed

#### Scenario: Report shows deletion candidates
- **WHEN** a file's description terms all appear in at least one other file
- **THEN** it appears in the deletion candidates section with the covering file named

#### Scenario: Clean repository produces empty report
- **WHEN** `orga memory defrag` is called and the repository has no oversized, duplicate, or deletable files
- **THEN** the command exits with code 0 and prints nothing (or empty arrays with `--json`)

#### Scenario: JSON output
- **WHEN** `orga memory defrag --json` is called
- **THEN** output is a valid JSON object with keys `oversized`, `duplicates`, `deletion_candidates`
