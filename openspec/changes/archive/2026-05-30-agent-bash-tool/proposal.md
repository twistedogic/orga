## Why

Agents working with code or scripts in a ticket workspace can only read and write files today — they cannot execute anything. A `bash` tool gives the agent the ability to run arbitrary shell commands (build, test, lint, git, etc.) in the context of the ticket's workspace, making it possible to automate real development work.

## What Changes

- Add a `bash(command)` tool to the agent tool set
- The tool runs `sh -c <command>` with `cwd` set to the ticket's workspace directory
- The workspace directory is created if it does not exist
- Returns a structured result: `{ stdout, stderr, exit_code }`
- A 120-second timeout is enforced; commands that exceed it are killed and an error is returned
- The tool requires `[workspace]` to be configured; it errors otherwise
- The tool is available to both main agents and subagents (via their `tools` list)
- No dry-run suppression — `bash` always executes when called

## Capabilities

### New Capabilities
- `agent-bash-tool`: Shell command execution in the ticket workspace with structured output and timeout enforcement

### Modified Capabilities
- `agent-tools`: Add `bash(command)` to the file tools table and scenario coverage

## Impact

- `src/agent/tools.rs` — new `dispatch_bash` handler and tool definition
- `openspec/specs/agent-tools/spec.md` — updated to include `bash` in the file tools requirement
- `openspec/specs/agent-bash-tool/spec.md` — new spec
