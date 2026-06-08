# context-repository Specification

## Purpose
Git-backed, topic-organized memory repository for cross-ticket context. Stores markdown files by topic with git versioning, progressive disclosure via file tree index, and always-loaded system/ files.

## Requirements

### Requirement: Git-backed filesystem repository
The system SHALL maintain a context repository as a local filesystem directory initialized as a git repository. The default path SHALL be `~/.orga/memory/`. An alternative path SHALL be configurable via `[memory] path` in config. The repository SHALL be created and initialized automatically on first use if it does not exist.

#### Scenario: First use initialization
- **WHEN** any memory operation is performed and the repository does not exist
- **THEN** the directory is created, initialized as a git repository, and the `system/` subdirectory is created with an initial `system/overview.md` stub file committed

#### Scenario: Existing repository is opened
- **WHEN** any memory operation is performed and the repository already exists
- **THEN** the existing repository is opened without modification

### Requirement: Markdown files with YAML frontmatter
Each file in the context repository SHALL be a UTF-8 markdown file. Each file SHOULD include a YAML frontmatter block at the top with at minimum a `description` field. The `description` field is a one-line summary of the file's contents used for navigation.

#### Scenario: Frontmatter present
- **WHEN** a memory file is written with frontmatter
- **THEN** the `description` field is extracted and included in the file tree index

#### Scenario: Frontmatter absent
- **WHEN** a memory file exists without frontmatter
- **THEN** the file is still listed in the tree index with an empty description

### Requirement: system/ directory always loaded
Files in the `system/` subdirectory SHALL be fully loaded into the agent's system prompt on every invocation. All other files SHALL only be loaded on explicit `memory_read` calls.

#### Scenario: system/ files injected into context
- **WHEN** an agent session begins
- **THEN** the full content of every file in `system/` is included in the system prompt under a `## Context Repository (pinned)` section

#### Scenario: Non-system files not auto-loaded
- **WHEN** an agent session begins and non-system files exist in the repository
- **THEN** those files are NOT included in the system prompt unless explicitly read via `memory_read`

### Requirement: File tree index always injected
The context repository file tree (all file paths and their frontmatter `description` fields) SHALL be rendered into every agent system prompt under a `## Context Repository` section, regardless of whether `system/` files are present.

#### Scenario: Tree index injected
- **WHEN** an agent session begins and the repository contains files
- **THEN** the system prompt includes a tree listing of all `.md` files with their descriptions

#### Scenario: Empty repository
- **WHEN** an agent session begins and the repository is empty
- **THEN** the system prompt includes a `## Context Repository` section noting the repository is empty

### Requirement: Every write is a git commit
Every `memory_write` operation SHALL produce a git commit in the repository with the topic path and the agent-provided `commit_msg` as the commit message. The commit author SHALL be the configured agent name.

#### Scenario: Write produces commit
- **WHEN** `memory_write` is called with a path, content, and commit message
- **THEN** the file is written and a git commit is created with the provided message

#### Scenario: Commit history is queryable
- **WHEN** `git log` is run on the repository
- **THEN** each memory write appears as a distinct commit with an informative message

### Requirement: Configurable defragmentation threshold
The defragmentation threshold SHALL be configurable via `[memory] defrag_file_threshold` (default: 20) and `[memory] defrag_size_threshold_kb` (default: 50) in config.

#### Scenario: Default thresholds apply
- **WHEN** no threshold config is provided
- **THEN** defragmentation is triggered at 20 files or 50KB total repository size
