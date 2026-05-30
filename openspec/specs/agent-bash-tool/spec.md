# agent-bash-tool Specification

## Purpose
Shell command execution in the ticket workspace with structured output and timeout enforcement.

## Requirements

### Requirement: bash tool
The agent loop SHALL expose a `bash(command)` tool when `[workspace]` is configured. The tool SHALL execute `sh -c <command>` with the working directory set to the ticket's workspace root. If the workspace directory does not exist it SHALL be created before execution. The tool SHALL return a JSON object with fields `stdout` (string), `stderr` (string), and `exit_code` (integer). A hard timeout of 120 seconds SHALL be enforced; if the command exceeds the timeout it SHALL be killed and the tool SHALL return `exit_code: -1` with `stderr` set to `"timeout: command exceeded 120s"`. The tool is NOT subject to dry-run suppression — it always executes when called. The tool SHALL be available to both main agents and subagents via their `tools` list.

#### Scenario: bash tool executes command and returns structured output
- **WHEN** the LLM calls `bash(command: "echo hello")`
- **THEN** the tool returns `{ "stdout": "hello\n", "stderr": "", "exit_code": 0 }`

#### Scenario: bash captures stderr and non-zero exit code
- **WHEN** the LLM calls `bash(command: "ls /nonexistent")`
- **THEN** `exit_code` is non-zero and `stderr` contains the error message from the shell

#### Scenario: bash runs in ticket workspace directory
- **WHEN** the LLM calls `bash(command: "pwd")`
- **THEN** `stdout` contains the absolute path to the ticket's workspace root

#### Scenario: bash creates workspace directory if missing
- **WHEN** no files have been written to the workspace yet and the LLM calls `bash(command: "pwd")`
- **THEN** the workspace directory is created and the command succeeds

#### Scenario: bash enforces 120-second timeout
- **WHEN** the LLM calls `bash(command: "sleep 300")`
- **THEN** the process is killed after 120 seconds and the tool returns `{ "exit_code": -1, "stderr": "timeout: command exceeded 120s", "stdout": "" }`

#### Scenario: bash requires workspace configured
- **WHEN** `[workspace]` is not configured and the LLM calls `bash(command: "ls")`
- **THEN** the tool returns an error: `"error: workspace not configured"`

#### Scenario: bash executes in dry-run mode
- **WHEN** dry-run is active and the LLM calls `bash(command: "echo hi")`
- **THEN** the command executes normally and returns structured output (bash is not suppressed in dry-run)
