## 1. Dependencies

- [x] 1.1 Add `serde_yaml` to `Cargo.toml`

## 2. Markdown Agent Loader

- [x] 2.1 Create `load_markdown_agents(agents_dir: &Path, logger: &Logger) -> Vec<SubagentConfig>` in `src/config.rs`
- [x] 2.2 Implement frontmatter split: detect `---\n...\n---\n` pattern, extract YAML block and body
- [x] 2.3 Define `SubagentFrontmatter` serde struct for YAML deserialization (`description`, `tools`, `skills`, `max_actions`)
- [x] 2.4 Deserialize frontmatter via `serde_yaml`; skip file with warning if `description` is missing or YAML is malformed
- [x] 2.5 Derive subagent `name` from file stem; set `system_prompt` from body

## 3. Integration

- [x] 3.1 In `AppConfig::load`, after TOML parse, derive `agents_dir` from config file parent and call `load_markdown_agents`
- [x] 3.2 Append returned agents to `self.subagents`

## 4. Tests

- [x] 4.1 Unit test: valid markdown file with all frontmatter fields loads correctly
- [x] 4.2 Unit test: markdown file with only `description` loads with defaults
- [x] 4.3 Unit test: missing `description` skips file without panic
- [x] 4.4 Unit test: malformed YAML skips file without panic
- [x] 4.5 Unit test: missing `agents/` directory returns empty vec without error
