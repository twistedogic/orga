## Context

The agent tool set in `src/agent/tools.rs` currently includes `read_file`, `write_file`, `list_files`, and `bash`. All four operate inside the per-ticket workspace directory provided by `WorkspaceStore`. The `bash` tool runs arbitrary shell commands with the workspace as `cwd`, making `read_file`, `write_file`, and `list_files` fully redundant — agents can use `cat`, `tee`/`echo >`, and `find`/`ls -R` instead.

The `VALID_TOOLS` whitelist in `src/config.rs` controls which tools subagents may be granted. It currently includes the three file tools but not `bash`.

## Goals / Non-Goals

**Goals:**
- Remove `read_file`, `write_file`, `list_files` dispatch arms, arg structs, tool definitions, and tests
- Add `"bash"` to `VALID_TOOLS` in config validation
- Update `openspec/specs/agent-workspace/spec.md` to reflect the new tool surface

**Non-Goals:**
- Removing `WorkspaceStore` or `workspace.rs` — `bash` still needs `ticket_root_path` for its `cwd`
- Changing `bash` behavior
- Migrating any existing agent configs (breaking change, users update their own configs)

## Decisions

### Remove all three file tools, no deprecation period
The three tools were added shortly before `bash`. No production deployments are known to rely on them. A clean removal is simpler than a deprecation flag.

### Keep WorkspaceStore
`bash` still uses `ws.ticket_root_path()` to resolve `cwd`. The module stays; only the tool dispatch layer changes.

### Add `bash` to VALID_TOOLS
It was missing from the whitelist — an oversight from when it was added. This is a bug fix alongside the removal.

## Risks / Trade-offs

- **Breaking change for subagent configs** that list `read_file`, `write_file`, or `list_files` in their `tools` array → Config validation will now reject those entries. Mitigation: documented as breaking in the proposal; users replace with `bash`.
- **bash is less constrained** than the file tools — agents can run arbitrary commands, not just file I/O. This was already true before; this change doesn't introduce the risk, just removes the alternative. Mitigation: none needed; `bash` access is already gated by subagent config.
