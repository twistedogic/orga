## MODIFIED Requirements

### Requirement: Topic-organized context repository
The system SHALL provide a local context repository organized as topic-named markdown files in a git-backed directory. The repository SHALL persist across CLI invocations. The default path SHALL be `~/.orga/memory/`, overridable via `[memory] path` in config. The implementation SHALL live in `src/memory/context_repo.rs` and be re-exported from `src/memory/mod.rs` as `crate::memory::ContextRepository`.

#### Scenario: Import path unchanged
- **WHEN** any module imports `crate::memory::ContextRepository`
- **THEN** the import resolves correctly after the module split

#### Scenario: Repository persists between invocations
- **WHEN** the sleep-time agent writes a topic file and the CLI exits
- **THEN** a subsequent CLI invocation can read the same file via `orga memory read`

## ADDED Requirements

### Requirement: Compaction store module
`CompactionStore` SHALL live in `src/memory/compaction.rs` and be re-exported from `src/memory/mod.rs` as `crate::memory::CompactionStore`.

#### Scenario: Import path unchanged after split
- **WHEN** any module imports `crate::memory::CompactionStore`
- **THEN** the import resolves correctly

### Requirement: Todo store module
`TodoStore` SHALL live in `src/memory/todo.rs` and be re-exported from `src/memory/mod.rs` as `crate::memory::TodoStore`.

#### Scenario: Import path unchanged after split
- **WHEN** any module imports `crate::memory::TodoStore`
- **THEN** the import resolves correctly
