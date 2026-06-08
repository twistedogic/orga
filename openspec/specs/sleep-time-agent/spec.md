# sleep-time-agent Specification

## Purpose
Background reflection agent that runs after done() to persist cross-ticket learnings into the context repository, and triggers defragmentation when the repository exceeds configured thresholds.

## Requirements

### Requirement: Sleep-time agent triggered after done()
The agent loop SHALL spawn a sleep-time reflection agent asynchronously after `done()` resolves successfully. The sleep-time agent SHALL receive the completed ticket's full context (title, description, comments) and the current memory file tree index, and SHALL be prompted to persist cross-ticket learnings into topic files in the context repository.

#### Scenario: done() triggers reflection
- **WHEN** the main agent calls `done()` and the board operation succeeds
- **THEN** a sleep-time agent is spawned before the ticket cycle exits

#### Scenario: done() failure does not trigger reflection
- **WHEN** the main agent calls `done()` and the board operation fails
- **THEN** no sleep-time agent is spawned

#### Scenario: skip() does not trigger reflection
- **WHEN** the main agent calls `skip()`
- **THEN** no sleep-time agent is spawned

### Requirement: Sleep-time agent writes to context repository
The sleep-time agent SHALL use `memory_read`, `memory_write`, and `memory_list` tools to update the context repository with learnings from the completed ticket. It SHALL write only cross-ticket-valuable information (themes, patterns, conventions, people context) — not ticket-specific facts.

#### Scenario: New theme discovered
- **WHEN** the sleep-time agent identifies a recurring architectural theme from the completed ticket
- **THEN** it creates or updates a file under `themes/` in the context repository with a commit message referencing the ticket

#### Scenario: Nothing worth persisting
- **WHEN** the sleep-time agent determines the ticket contained no cross-ticket-valuable learnings
- **THEN** it makes no writes and exits cleanly

### Requirement: Sleep-time agent error isolation
Errors during the sleep-time reflection pass SHALL be logged but SHALL NOT affect the ticket cycle outcome. The `done()` call is already complete; reflection is best-effort.

#### Scenario: Sleep-time agent LLM call fails
- **WHEN** the sleep-time agent's LLM call errors
- **THEN** the error is logged and the ticket cycle exits normally

### Requirement: Defragmentation triggered by threshold
After writing new learnings, the sleep-time agent SHALL check the repository against the configured thresholds. If the total number of `.md` files exceeds `defrag_file_threshold` OR the total size exceeds `defrag_size_threshold_kb`, the sleep-time agent SHALL run a defragmentation pass.

#### Scenario: Threshold not exceeded
- **WHEN** the repository has fewer files and smaller total size than both thresholds
- **THEN** no defragmentation pass runs

#### Scenario: File count threshold exceeded
- **WHEN** the repository has more `.md` files than `defrag_file_threshold`
- **THEN** a defragmentation pass runs after the reflection writes

#### Scenario: Size threshold exceeded
- **WHEN** the total size of `.md` files exceeds `defrag_size_threshold_kb`
- **THEN** a defragmentation pass runs after the reflection writes

### Requirement: Defragmentation reorganizes the repository
The defragmentation pass SHALL reorganize the context repository by splitting oversized files and merging files with overlapping content. Merged originals SHALL be deleted using `memory_delete` after their content is consolidated. The defrag agent SHALL NOT reorganize the folder hierarchy or rename directories. It SHALL commit each file change individually.

Available tools during defrag: `memory_list`, `memory_read`, `memory_write`, `memory_delete`.

#### Scenario: Oversized file split
- **WHEN** a file exceeds a reasonable size (e.g. 200 lines) and covers multiple distinct topics
- **THEN** the defrag pass splits it into focused files, each committed separately

#### Scenario: Duplicate files merged and originals deleted
- **WHEN** two files cover substantially the same topic
- **THEN** the defrag pass writes a merged file, then deletes the originals via `memory_delete`

#### Scenario: Deletion blocked during defrag
- **WHEN** the defrag agent calls `memory_delete` on a file whose description terms are not covered elsewhere
- **THEN** the delete is blocked and the agent receives an error — it SHALL NOT delete that file

#### Scenario: Hierarchy reorganization not performed
- **WHEN** the defrag agent runs
- **THEN** it does NOT rename folders or restructure the directory hierarchy
