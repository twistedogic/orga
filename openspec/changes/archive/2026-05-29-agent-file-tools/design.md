## Context

Agents today can persist named text blobs via `commit_artifact` / `get_artifact`, which are backed by a git store and treated as versioned outputs. There is no mechanism for general-purpose file I/O during task execution — no way to read an input file, stage intermediate work, or write output to a well-known path before deciding to commit it as an artifact.

The workspace is a local filesystem sandbox, one directory per ticket, rooted at a configurable base path. It is entirely separate from the artifact store: no git, no commits, no versioning. It is a plain working area.

## Goals / Non-Goals

**Goals:**
- Provide `read_file`, `write_file`, `list_files` tools to agents
- Each tool operates within a per-ticket workspace directory: `<workspace.path>/<ticket_id>/`
- Path traversal attacks are rejected with a clear error
- Binary file reads return an error
- `list_files` returns a flat list of relative paths
- Workspace is opt-in via `[workspace]` config section; tools fail gracefully if not configured
- `write_file` creates intermediate directories as needed

**Non-Goals:**
- No versioning, history, or git integration
- No delete or rename tools (can be added later)
- No binary file write support (text only for now)
- No size limits or quotas
- No sharing between tickets

## Decisions

### 1. New `WorkspaceStore` struct, not a trait

**Decision**: Implement as a plain struct `WorkspaceStore { base: PathBuf }`, not a trait.

**Rationale**: Unlike `ArtifactStore` (which has a git backend today and may have others), there is only one meaningful workspace backend: the local filesystem. A trait would add abstraction without benefit. If remote backends are ever needed, the struct can be refactored then.

**Alternative considered**: Trait like `ArtifactStore`. Rejected — premature abstraction.

### 2. Path safety via `canonicalize` + prefix check

**Decision**: Resolve the requested path relative to the ticket workspace, then call `std::fs::canonicalize` (after ensuring the directory exists) and verify the result starts with the ticket workspace root.

**Rationale**: Symlink traversal and `../` sequences are both neutralized. Simple string prefix checks are insufficient because `../` can escape after resolution.

**For `write_file`**: Create the file first (after building the absolute path by joining and normalizing without canonicalize, since the file may not exist yet), then verify the parent directory is within bounds.

**Alternative considered**: Stripping `..` components manually. Rejected — fragile and easy to get wrong.

### 3. Binary detection via UTF-8 validation

**Decision**: On `read_file`, attempt `String::from_utf8` on the bytes. If it fails, return `error: file contains binary content`.

**Rationale**: Simple, zero-dependency, consistent with the "text only" constraint. No need for MIME sniffing.

### 4. `list_files` returns relative paths, recursive, flat

**Decision**: Walk the entire ticket workspace directory tree and return all file paths as a newline-separated list of paths relative to the ticket workspace root.

**Rationale**: Simple to consume by an LLM. A path like `output/report.md` can be passed directly back to `read_file` or `write_file`.

**Alternative considered**: JSON array. Rejected — plain newline-separated text is consistent with other tool return values.

### 5. `ToolContext` gains optional `workspace: Option<WorkspaceStore>`

**Decision**: Add `workspace: Option<WorkspaceStore>` to `ToolContext`. Tools check for `None` and return `error: workspace not configured`.

**Rationale**: Consistent with how `artifact_store: Option<Box<dyn ArtifactStore>>` is handled. No change to callers that don't use workspace tools.

## Risks / Trade-offs

- **Disk space**: Agents can write arbitrarily large files. No quota enforcement. → Acceptable for now; operators control what tools agents can access via `tools` list in subagent config.
- **Workspace persistence**: Files accumulate indefinitely; there is no automatic cleanup. → Out of scope; a future `clean_workspace` command or CLI subcommand could address this.
- **Ticket ID as directory name**: Ticket IDs from Trello/Linear may contain characters unsafe for filesystem paths (slashes, colons). → Sanitize ticket ID when constructing the path: replace `/`, `:`, and other unsafe characters with `_`.
