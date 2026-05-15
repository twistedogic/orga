## 1. Config — data structures and validation

- [x] 1.1 Add `WorkflowEntry` struct to `src/config.rs` with fields: `column: String`, `prompt: Option<String>`, `prompt_file: Option<String>`
- [x] 1.2 Add `workflow: Vec<WorkflowEntry>` field (with `#[serde(default)]`) to `AppConfig`
- [x] 1.3 Add validation in `AppConfig::validate()`: for each workflow entry, enforce exactly one of `prompt`/`prompt_file`; hard-fail with `ConfigError` listing the column name on violation
- [x] 1.4 Add `prompt_file` resolution in `AppConfig::validate()`: expand tilde, read file contents, hard-fail with `ConfigError` if file does not exist or cannot be read
- [x] 1.5 Store resolved prompt text back on the entry so lookup is pure (no file I/O after load)

## 2. Config — lookup

- [x] 2.1 Add `AppConfig::workflow_prompt(&self, list_name: &str) -> Option<&str>` method: iterates `workflow`, matches `column` case-insensitively, returns resolved prompt text

## 3. CLI — ticket show output

- [x] 3.1 In `TicketCommands::Show` handler in `src/main.rs`, call `config.workflow_prompt(&ticket.summary.list_name)`
- [x] 3.2 JSON path: when `Some(prompt)`, add `"workflow_prompt": prompt` field to the serialized ticket JSON object
- [x] 3.3 Human-readable path: when `Some(prompt)`, append `\n## Workflow\n{prompt}` block after the existing ticket output

## 4. Tests

- [x] 4.1 Unit test in `src/config.rs`: entry with inline `prompt` loads correctly
- [x] 4.2 Unit test: entry with `prompt_file` pointing to a temp file loads and reads content correctly
- [x] 4.3 Unit test: entry with `prompt_file` pointing to non-existent path fails at load with `ConfigError`
- [x] 4.4 Unit test: entry with both `prompt` and `prompt_file` fails at load with `ConfigError`
- [x] 4.5 Unit test: entry with neither `prompt` nor `prompt_file` fails at load with `ConfigError`
- [x] 4.6 Unit test: `workflow_prompt` returns `Some` for exact-case match
- [x] 4.7 Unit test: `workflow_prompt` returns `Some` for case-insensitive match
- [x] 4.8 Unit test: `workflow_prompt` returns `None` when no entry matches

## 5. Documentation

- [x] 5.1 Update `skills/orga/SKILL.md` config reference block to include `[[workflow]]` section with both `prompt` and `prompt_file` examples
