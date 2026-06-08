## MODIFIED Requirements

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
