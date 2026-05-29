## ADDED Requirements

### Requirement: File tools
The agent loop SHALL expose `read_file`, `write_file`, and `list_files` tools when `[workspace]` is configured. These tools SHALL be available to subagents via their `tools` list. In dry-run mode, `write_file` SHALL be logged but not executed; `read_file` and `list_files` SHALL execute normally.

| Tool | Mutating | Maps to |
|------|----------|---------|
| `read_file(path)` | no | `workspace_store.read()` |
| `write_file(path, content)` | yes | `workspace_store.write()` |
| `list_files()` | no | `workspace_store.list()` |

#### Scenario: read_file executes in dry-run
- **WHEN** dry-run is active and the LLM calls `read_file(path: "notes.md")`
- **THEN** the file is read and its content returned (read tools are not suppressed)

#### Scenario: write_file suppressed in dry-run
- **WHEN** dry-run is active and the LLM calls `write_file(path: "out.md", content: "...")`
- **THEN** the action is logged but not executed; a dry-run notice is returned as the tool result

#### Scenario: list_files executes in dry-run
- **WHEN** dry-run is active and the LLM calls `list_files()`
- **THEN** the file listing is returned normally (read tools are not suppressed)
