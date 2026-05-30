## Why

The `read_file`, `write_file`, and `list_files` agent tools are redundant now that `bash` is available — agents can use `cat`, `echo`, `ls`, and any other shell command to interact with the workspace. Keeping three special-purpose file tools adds surface area, maintenance burden, and LLM token overhead for no additional capability.

## What Changes

- **BREAKING** Remove `read_file` agent tool
- **BREAKING** Remove `write_file` agent tool
- **BREAKING** Remove `list_files` agent tool
- Remove `ReadFileArgs`, `WriteFileArgs` dispatch functions and tool definitions from `src/agent/tools.rs`
- Remove `read_file`, `write_file`, `list_files` from `VALID_TOOLS` whitelist in `src/config.rs`; add `bash`
- Update `openspec/specs/agent-workspace/spec.md` to reflect `bash` as the sole workspace interaction tool

## Capabilities

### New Capabilities
- none

### Modified Capabilities
- `agent-workspace`: Remove `read_file`, `write_file`, `list_files` tool requirements; document that `bash` is the sole file I/O mechanism in the workspace

## Impact

- `src/agent/tools.rs` — dispatch arms, arg structs, dispatch functions, tool definitions, tests for the three removed tools
- `src/config.rs` — `VALID_TOOLS` constant
- `openspec/specs/agent-workspace/spec.md` — spec rewrite
- Any subagent configs using the removed tool names will fail config validation (breaking change)
