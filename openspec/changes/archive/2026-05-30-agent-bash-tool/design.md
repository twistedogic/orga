## Context

The agent currently has `read_file`, `write_file`, and `list_files` for interacting with the ticket workspace. These are sufficient for document-style work but not for executing code. Adding `bash` completes the workspace tool set for software development tasks.

The workspace root per ticket is `<base>/<sanitized_ticket_id>/`. This directory already exists as a concept in `WorkspaceStore`. The bash tool simply sets `cwd` to that path when spawning a process.

## Goals / Non-Goals

**Goals:**
- Execute arbitrary shell commands in the ticket workspace directory
- Return structured output: `{ stdout, stderr, exit_code }`
- Enforce a 120-second timeout; kill and report on breach
- Auto-create the workspace directory if it does not exist (consistent with `write_file`)
- Gate on `[workspace]` being configured — same as file tools

**Non-Goals:**
- Sandboxing or restricting what commands can run (agent is trusted)
- Dry-run suppression
- Configurable timeout (fixed at 120s)
- Streaming output

## Decisions

### `sh -c <command>` vs direct exec
Use `sh -c <command>` so the agent can use pipes, redirection, env vars, and compound commands without the tool needing to parse or understand shell syntax. Alternative was `execvp` with explicit argv — rejected because it prevents shell features the agent will want.

### Structured return: JSON `{ stdout, stderr, exit_code }`
Return all three fields so the agent can reason about failures explicitly (non-zero exit code + stderr) without the tool itself treating non-zero as an error. Alternative was to return an error tool result on non-zero — rejected because the agent should decide what counts as a failure.

### 120s hard timeout
Implemented via `tokio::time::timeout` wrapping `tokio::process::Command`. On timeout, the child process is killed (`kill()` + `wait()`). Returns `exit_code: -1` with stderr set to `"timeout: command exceeded 120s"`. Fixed value avoids config complexity for a first implementation.

### Requires `[workspace]` configured
Without a workspace root there is no well-defined `cwd` to offer. Rather than fall back to the process's cwd (surprising, potentially dangerous), the tool returns an error. This is consistent with the other workspace tools.

## Risks / Trade-offs

- **Unbounded side effects** — bash can do anything: network calls, delete files outside workspace, spawn daemons. → Accepted; agent is trusted by design.
- **Long-running processes within timeout** — 120s may be too short for `cargo build` on a cold cache. → Acceptable for now; timeout value can be revisited.
- **stdout/stderr truncation** — very verbose commands could produce large outputs that inflate the LLM context. → Not addressed in this change; can be added later with a byte cap.
- **Process leaks on panic** — if the Rust process panics after spawning a child but before the timeout fires, the child may linger. → Low probability; acceptable for CLI use.

## Open Questions

- Should `bash` be added to the narrowed main-agent tool set, or only available to subagents? Currently proposed as available to both.
