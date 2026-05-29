use std::path::{Path, PathBuf};
use std::{env, fs};

use serde::Deserialize;

use crate::error::OrgaError;
use crate::logging::Logger;

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct BoardConfig {
    pub backend: String,
}

#[derive(Debug, Deserialize)]
pub struct LinearConfig {
    pub api_key: String,
    pub team_id: String,
}

#[derive(Debug, Deserialize)]
pub struct TrelloConfig {
    pub api_key: String,
    pub token: String,
    pub member_id: String,
    pub board_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MemoryConfig {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowEntry {
    pub column: String,
    pub prompt: Option<String>,
    pub prompt_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceConfig {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct SkillsConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubagentConfig {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    pub model: Option<String>,
    pub max_actions: Option<usize>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubagentFrontmatter {
    description: Option<String>,
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
    max_actions: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub file: Option<String>,
    pub debug: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub endpoint: Option<String>,
    pub poll_interval_secs: Option<u64>,
    pub max_actions_per_ticket: Option<usize>,
}

impl LlmConfig {
    pub fn poll_interval_secs(&self) -> u64 {
        self.poll_interval_secs.unwrap_or(60)
    }

    pub fn max_actions_per_ticket(&self) -> usize {
        self.max_actions_per_ticket.unwrap_or(10)
    }

}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub agent: AgentConfig,
    pub board: BoardConfig,
    pub trello: Option<TrelloConfig>,
    pub linear: Option<LinearConfig>,
    pub memory: Option<MemoryConfig>,
    pub logging: Option<LoggingConfig>,
    pub llm: Option<LlmConfig>,
    #[serde(default)]
    pub workflow: Vec<WorkflowEntry>,
    pub comment_compaction_threshold: Option<usize>,
    pub skills: Option<SkillsConfig>,
    pub workspace: Option<WorkspaceConfig>,
    #[serde(default)]
    pub subagents: Vec<SubagentConfig>,
}

impl AppConfig {
    pub fn try_load(path: &Path) -> Option<Self> {
        Self::load(path).ok()
    }

    pub fn load(path: &Path) -> Result<Self, OrgaError> {
        let content = fs::read_to_string(path).map_err(|e| {
            OrgaError::ConfigError(format!(
                "cannot read config at {}: {}",
                path.display(),
                e
            ))
        })?;
        let mut config: AppConfig = toml::from_str(&content)
            .map_err(|e| OrgaError::ConfigError(format!("invalid config: {e}")))?;
        if content.contains("[artifact]") || content.contains("[artifact.git]") {
            return Err(OrgaError::ConfigError(
                "[artifact] section is no longer supported; use [workspace] for per-ticket file storage".into(),
            ));
        }
        if let Some(parent) = path.parent() {
            let agents_dir = parent.join("agents");
            let logger = config.logger();
            let md_agents = load_markdown_agents(&agents_dir, &logger);
            config.subagents.extend(md_agents);
        }
        config.validate()?;
        Ok(config)
    }

    pub fn resolve_path(explicit: Option<&str>) -> PathBuf {
        if let Some(p) = explicit {
            return PathBuf::from(p);
        }
        if let Ok(p) = env::var("ORGA_CONFIG") {
            return PathBuf::from(p);
        }
        default_config_path()
    }

    pub fn memory_db_path(&self) -> PathBuf {
        let raw = self
            .memory
            .as_ref()
            .and_then(|m| m.path.as_deref())
            .unwrap_or("~/.orga/memory.db");
        expand_tilde(raw)
    }

    fn validate(&mut self) -> Result<(), OrgaError> {
        const SUPPORTED: &[&str] = &["trello", "linear"];
        if !SUPPORTED.contains(&self.board.backend.as_str()) {
            return Err(OrgaError::ConfigError(format!(
                "unsupported backend '{}'. Supported backends: {}",
                self.board.backend,
                SUPPORTED.join(", ")
            )));
        }
        if self.board.backend == "trello" && self.trello.is_none() {
            return Err(OrgaError::ConfigError(
                "backend is 'trello' but [trello] section is missing from config".into(),
            ));
        }
        if self.board.backend == "linear" && self.linear.is_none() {
            return Err(OrgaError::ConfigError(
                "backend is 'linear' but [linear] section is missing from config".into(),
            ));
        }
        if self.board.backend == "trello"
            && let Some(ref t) = self.trello
                && t.board_id.is_empty() {
                    return Err(OrgaError::ConfigError(
                        "[trello] board_id is required".into(),
                    ));
                }
        if self.board.backend == "linear"
            && let Some(ref l) = self.linear
                && l.team_id.is_empty() {
                    return Err(OrgaError::ConfigError(
                        "[linear] team_id is required".into(),
                    ));
                }
        if let Some(ref llm) = self.llm {
            const SUPPORTED_PROVIDERS: &[&str] = &["anthropic", "openai"];
            if !SUPPORTED_PROVIDERS.contains(&llm.provider.as_str()) {
                return Err(OrgaError::ConfigError(format!(
                    "[llm] unsupported provider '{}'. Supported providers: {}",
                    llm.provider,
                    SUPPORTED_PROVIDERS.join(", ")
                )));
            }
            if llm.api_key.is_empty() {
                return Err(OrgaError::ConfigError(
                    "[llm] api_key is required".into(),
                ));
            }
            if llm.model.is_empty() {
                return Err(OrgaError::ConfigError(
                    "[llm] model is required".into(),
                ));
            }
        }
        for entry in &mut self.workflow {
            match (&entry.prompt, &entry.prompt_file) {
                (Some(_), Some(_)) => {
                    return Err(OrgaError::ConfigError(format!(
                        "workflow entry for column '{}': specify either 'prompt' or 'prompt_file', not both",
                        entry.column
                    )));
                }
                (None, None) => {
                    return Err(OrgaError::ConfigError(format!(
                        "workflow entry for column '{}': must specify either 'prompt' or 'prompt_file'",
                        entry.column
                    )));
                }
                (None, Some(path)) => {
                    let expanded = expand_tilde(path);
                    let text = fs::read_to_string(&expanded).map_err(|e| {
                        OrgaError::ConfigError(format!(
                            "workflow entry for column '{}': cannot read prompt_file '{}': {}",
                            entry.column,
                            expanded.display(),
                            e
                        ))
                    })?;
                    entry.prompt = Some(text);
                    entry.prompt_file = None;
                }
                (Some(_), None) => {}
            }
        }

        // Validate subagents
        const VALID_TOOLS: &[&str] = &[
            "comment", "move_ticket", "assign", "create_sub", "set_memory",
            "compact", "done", "skip",
            "dispatch", "return", "read_file", "write_file", "list_files",
        ];
        let mut seen_names = std::collections::HashSet::new();
        for sub in &self.subagents {
            if !seen_names.insert(sub.name.clone()) {
                return Err(OrgaError::ConfigError(format!(
                    "[subagents] duplicate subagent name '{}'",
                    sub.name
                )));
            }
            for tool in &sub.tools {
                if !VALID_TOOLS.contains(&tool.as_str()) {
                    return Err(OrgaError::ConfigError(format!(
                        "[subagents] subagent '{}' references unknown tool '{}'",
                        sub.name, tool
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn logger(&self) -> Logger {
        let path = self
            .logging
            .as_ref()
            .and_then(|l| l.file.as_deref())
            .unwrap_or("~/.orga/orga.log");
        let debug = self
            .logging
            .as_ref()
            .and_then(|l| l.debug)
            .unwrap_or(false);
        Logger::new(&expand_tilde(path), debug)
    }

    pub fn workflow_prompt(&self, list_name: &str) -> Option<&str> {
        let lower = list_name.to_lowercase();
        self.workflow
            .iter()
            .find(|e| e.column.to_lowercase() == lower)
            .and_then(|e| e.prompt.as_deref())
    }

    pub fn compaction_threshold(&self) -> usize {
        self.comment_compaction_threshold.unwrap_or(5)
    }

    pub fn llm_config(&self) -> Result<&LlmConfig, OrgaError> {
        self.llm.as_ref().ok_or_else(|| {
            OrgaError::ConfigError(
                "[llm] section is required for `orga agent` but is missing from config".into(),
            )
        })
    }

    pub fn skills_path(&self) -> Option<PathBuf> {
        self.skills.as_ref().map(|s| expand_tilde(&s.path))
    }

    pub fn workspace_base_path(&self) -> Option<PathBuf> {
        self.workspace.as_ref().map(|w| expand_tilde(&w.path))
    }

}

fn default_config_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".orga").join("config.toml")
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(rest)
    } else {
        PathBuf::from(path)
    }
}

fn load_markdown_agents(agents_dir: &Path, logger: &Logger) -> Vec<SubagentConfig> {
    let entries = match fs::read_dir(agents_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut agents = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                logger.warn(&format!("[agents] failed to read {}: {e}", path.display()));
                continue;
            }
        };

        match parse_markdown_agent(&name, &content) {
            Ok(agent) => agents.push(agent),
            Err(e) => {
                logger.warn(&format!("[agents] skipping {}: {e}", path.display()));
            }
        }
    }

    agents
}

fn parse_markdown_agent(name: &str, content: &str) -> Result<SubagentConfig, String> {
    let (frontmatter_str, body) = split_frontmatter(content)?;

    let fm: SubagentFrontmatter = serde_yaml::from_str(frontmatter_str)
        .map_err(|e| format!("invalid frontmatter YAML: {e}"))?;

    let description = fm
        .description
        .filter(|d| !d.is_empty())
        .ok_or_else(|| "missing required field: description".to_string())?;

    Ok(SubagentConfig {
        name: name.to_string(),
        description,
        tools: fm.tools,
        skills: fm.skills,
        model: None,
        max_actions: fm.max_actions,
        system_prompt: if body.trim().is_empty() {
            None
        } else {
            Some(body.trim().to_string())
        },
    })
}

fn split_frontmatter(content: &str) -> Result<(&str, &str), String> {
    if !content.starts_with("---") {
        return Err("missing frontmatter: file must start with '---'".to_string());
    }
    let after_first = &content[3..];
    let rest = after_first.strip_prefix('\n').unwrap_or(after_first);
    let close = rest
        .find("\n---")
        .ok_or_else(|| "missing closing '---' in frontmatter".to_string())?;
    let yaml = &rest[..close];
    let body = &rest[close + 4..]; // skip "\n---"
    let body = body.strip_prefix('\n').unwrap_or(body);
    Ok((yaml, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_config(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    const VALID_CONFIG: &str = r#"
[agent]
name = "agent-1"

[board]
backend = "trello"

[trello]
api_key = "key"
token = "tok"
member_id = "abc123"
board_id = "board-xyz"
"#;

    #[test]
    fn try_load_returns_none_for_missing_file() {
        assert!(AppConfig::try_load(Path::new("/nonexistent/config.toml")).is_none());
    }

    #[test]
    fn try_load_returns_none_for_invalid_toml() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"not valid toml [[[").unwrap();
        assert!(AppConfig::try_load(f.path()).is_none());
    }

    #[test]
    fn try_load_returns_some_for_valid_config() {
        let f = write_config(VALID_CONFIG);
        assert!(AppConfig::try_load(f.path()).is_some());
    }

    #[test]
    fn valid_config_loads() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.trello.unwrap().member_id, "abc123");
        assert_eq!(cfg.board.backend, "trello");
    }

    #[test]
    fn unknown_backend_fails() {
        let content = VALID_CONFIG.replace("backend = \"trello\"", "backend = \"notion\"");
        let f = write_config(&content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("unsupported backend"));
    }

    #[test]
    fn missing_trello_section_fails() {
        let content = r#"
[agent]
name = "agent-1"

[board]
backend = "trello"
"#;
        let f = write_config(content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("[trello] section"));
    }

    #[test]
    fn missing_config_file_errors() {
        let err = AppConfig::load(Path::new("/nonexistent/config.toml")).unwrap_err();
        assert!(err.to_string().contains("cannot read config"));
    }

    #[test]
    fn memory_db_path_default() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        let p = cfg.memory_db_path();
        assert!(p.ends_with(".orga/memory.db"));
    }

    #[test]
    fn memory_db_path_custom() {
        let content = format!("{VALID_CONFIG}\n[memory]\npath = \"/tmp/test.db\"\n");
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.memory_db_path(), PathBuf::from("/tmp/test.db"));
    }

    #[test]
    fn workflow_inline_prompt_loads() {
        let content = format!(
            "{VALID_CONFIG}\n[[workflow]]\ncolumn = \"To Do\"\nprompt = \"Enter explore mode.\"\n"
        );
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.workflow.len(), 1);
        assert_eq!(cfg.workflow[0].prompt.as_deref(), Some("Enter explore mode."));
    }

    #[test]
    fn workflow_prompt_file_loads() {
        let mut prompt_file = NamedTempFile::new().unwrap();
        prompt_file.write_all(b"Think deeply.").unwrap();
        let path = prompt_file.path().to_str().unwrap().to_owned();
        let content = format!(
            "{VALID_CONFIG}\n[[workflow]]\ncolumn = \"To Do\"\nprompt_file = \"{path}\"\n"
        );
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.workflow[0].prompt.as_deref(), Some("Think deeply."));
    }

    #[test]
    fn workflow_prompt_file_missing_fails() {
        let content = format!(
            "{VALID_CONFIG}\n[[workflow]]\ncolumn = \"To Do\"\nprompt_file = \"/nonexistent/prompt.md\"\n"
        );
        let f = write_config(&content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("cannot read prompt_file"));
    }

    #[test]
    fn workflow_both_prompt_and_prompt_file_fails() {
        let content = format!(
            "{VALID_CONFIG}\n[[workflow]]\ncolumn = \"To Do\"\nprompt = \"foo\"\nprompt_file = \"/some/path\"\n"
        );
        let f = write_config(&content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("not both"));
    }

    #[test]
    fn workflow_neither_prompt_nor_prompt_file_fails() {
        let content = format!(
            "{VALID_CONFIG}\n[[workflow]]\ncolumn = \"To Do\"\n"
        );
        let f = write_config(&content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("must specify either"));
    }

    #[test]
    fn workflow_prompt_exact_case_match() {
        let content = format!(
            "{VALID_CONFIG}\n[[workflow]]\ncolumn = \"To Do\"\nprompt = \"Explore.\"\n"
        );
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.workflow_prompt("To Do"), Some("Explore."));
    }

    #[test]
    fn workflow_prompt_case_insensitive_match() {
        let content = format!(
            "{VALID_CONFIG}\n[[workflow]]\ncolumn = \"To Do\"\nprompt = \"Explore.\"\n"
        );
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.workflow_prompt("to do"), Some("Explore."));
        assert_eq!(cfg.workflow_prompt("TO DO"), Some("Explore."));
    }

    #[test]
    fn workflow_prompt_no_match_returns_none() {
        let content = format!(
            "{VALID_CONFIG}\n[[workflow]]\ncolumn = \"To Do\"\nprompt = \"Explore.\"\n"
        );
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.workflow_prompt("In Progress"), None);
    }

    #[test]
    fn logging_section_absent_uses_defaults() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.logging.is_none());
        let logger = cfg.logger();
        drop(logger);
    }

    #[test]
    fn logging_section_with_custom_file() {
        let content = format!("{VALID_CONFIG}\n[logging]\nfile = \"/tmp/orga-test.log\"\n");
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.logging.as_ref().unwrap().file.as_deref(), Some("/tmp/orga-test.log"));
    }

    #[test]
    fn logging_debug_flag_propagated() {
        let content = format!("{VALID_CONFIG}\n[logging]\ndebug = true\n");
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.logging.as_ref().unwrap().debug, Some(true));
    }

    const VALID_LINEAR_CONFIG: &str = r#"
[agent]
name = "agent-1"

[board]
backend = "linear"

[linear]
api_key = "lin_api_abc123"
team_id = "team-xyz"
"#;

    #[test]
    fn linear_backend_recognized() {
        let f = write_config(VALID_LINEAR_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.board.backend, "linear");
    }

    #[test]
    fn valid_linear_config_loads() {
        let f = write_config(VALID_LINEAR_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.linear.unwrap().api_key, "lin_api_abc123");
    }

    #[test]
    fn missing_linear_section_fails() {
        let content = r#"
[agent]
name = "agent-1"

[board]
backend = "linear"
"#;
        let f = write_config(content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("[linear] section"));
    }

    const VALID_LLM_CONFIG: &str = r#"
[agent]
name = "agent-1"

[board]
backend = "trello"

[trello]
api_key = "key"
token = "tok"
member_id = "abc123"
board_id = "board-xyz"

[llm]
provider = "anthropic"
api_key = "sk-ant-test"
model = "claude-opus-4-5"
"#;

    #[test]
    fn valid_llm_section_loads() {
        let f = write_config(VALID_LLM_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        let llm = cfg.llm.as_ref().unwrap();
        assert_eq!(llm.provider, "anthropic");
        assert_eq!(llm.api_key, "sk-ant-test");
        assert_eq!(llm.model, "claude-opus-4-5");
    }

    #[test]
    fn llm_config_helper_returns_ok() {
        let f = write_config(VALID_LLM_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.llm_config().is_ok());
    }

    #[test]
    fn llm_config_helper_absent_section_returns_err() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        let err = cfg.llm_config().unwrap_err();
        assert!(err.to_string().contains("[llm]"));
    }

    #[test]
    fn llm_unknown_provider_fails() {
        let content = VALID_LLM_CONFIG.replace("provider = \"anthropic\"", "provider = \"unknown\"");
        let f = write_config(&content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("unsupported provider"));
    }

    #[test]
    fn llm_missing_api_key_fails() {
        let content = VALID_LLM_CONFIG.replace("api_key = \"sk-ant-test\"", "api_key = \"\"");
        let f = write_config(&content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("api_key is required"));
    }

    #[test]
    fn llm_missing_model_fails() {
        let content = VALID_LLM_CONFIG.replace("model = \"claude-opus-4-5\"", "model = \"\"");
        let f = write_config(&content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("model is required"));
    }

    #[test]
    fn llm_absent_section_does_not_affect_other_commands() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.llm.is_none());
        assert_eq!(cfg.board.backend, "trello");
    }

    #[test]
    fn llm_defaults() {
        let f = write_config(VALID_LLM_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        let llm = cfg.llm.as_ref().unwrap();
        assert_eq!(llm.poll_interval_secs(), 60);
        assert_eq!(llm.max_actions_per_ticket(), 10);
    }

    #[test]
    fn llm_endpoint_override() {
        let content = format!("{VALID_LLM_CONFIG}endpoint = \"https://proxy.example.com/v1\"\n");
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.llm.as_ref().unwrap().endpoint.as_deref(), Some("https://proxy.example.com/v1"));
    }

    #[test]
    fn llm_openai_provider_loads() {
        let content = VALID_LLM_CONFIG
            .replace("provider = \"anthropic\"", "provider = \"openai\"")
            .replace("api_key = \"sk-ant-test\"", "api_key = \"sk-openai-test\"");
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.llm.as_ref().unwrap().provider, "openai");
    }

    #[test]
    fn skills_section_absent_returns_none() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.skills.is_none());
        assert!(cfg.skills_path().is_none());
    }

    #[test]
    fn skills_section_present_loads_path() {
        let content = format!("{VALID_CONFIG}\n[skills]\npath = \"/tmp/skills\"\n");
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.skills.as_ref().unwrap().path, "/tmp/skills");
        assert_eq!(cfg.skills_path(), Some(PathBuf::from("/tmp/skills")));
    }

    #[test]
    fn skills_path_tilde_expanded() {
        let content = format!("{VALID_CONFIG}\n[skills]\npath = \"~/.orga/skills\"\n");
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        let p = cfg.skills_path().unwrap();
        assert!(!p.to_str().unwrap().contains('~'));
        assert!(p.ends_with(".orga/skills"));
    }

    #[test]
    fn subagent_config_parses_from_toml() {
        let content = format!("{VALID_CONFIG}\n[[subagents]]\nname = \"researcher\"\ndescription = \"Does research\"\ntools = [\"comment\", \"done\"]\n");
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.subagents.len(), 1);
        assert_eq!(cfg.subagents[0].name, "researcher");
        assert_eq!(cfg.subagents[0].tools, vec!["comment", "done"]);
    }

    #[test]
    fn subagent_config_with_optional_fields() {
        let content = format!("{VALID_CONFIG}\n[[subagents]]\nname = \"drafter\"\ndescription = \"Drafts content\"\ntools = [\"comment\"]\nskills = [\"writing\"]\nmodel = \"gpt-4o\"\nmax_actions = 20\n");
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.subagents[0].model.as_deref(), Some("gpt-4o"));
        assert_eq!(cfg.subagents[0].max_actions, Some(20));
        assert_eq!(cfg.subagents[0].skills, vec!["writing"]);
    }

    #[test]
    fn duplicate_subagent_name_fails_validation() {
        let content = format!("{VALID_CONFIG}\n[[subagents]]\nname = \"bot\"\ndescription = \"a\"\ntools = [\"done\"]\n[[subagents]]\nname = \"bot\"\ndescription = \"b\"\ntools = [\"done\"]\n");
        let f = write_config(&content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("duplicate subagent name"));
    }

    #[test]
    fn unknown_tool_name_in_subagent_fails_validation() {
        let content = format!("{VALID_CONFIG}\n[[subagents]]\nname = \"bot\"\ndescription = \"a\"\ntools = [\"fly\"]\n");
        let f = write_config(&content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("unknown tool"));
    }

    #[test]
    fn no_subagents_config_loads_fine() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.subagents.is_empty());
    }

    // --- markdown agent tests ---

    #[test]
    fn markdown_agent_all_fields_loads_correctly() {
        let content = "---\ndescription: Does research\ntools:\n  - comment\n  - done\nskills:\n  - rust\nmax_actions: 5\n---\nYou are a researcher.\n";
        let agent = parse_markdown_agent("researcher", content).unwrap();
        assert_eq!(agent.name, "researcher");
        assert_eq!(agent.description, "Does research");
        assert_eq!(agent.tools, vec!["comment", "done"]);
        assert_eq!(agent.skills, vec!["rust"]);
        assert_eq!(agent.max_actions, Some(5));
        assert_eq!(agent.system_prompt.as_deref(), Some("You are a researcher."));
    }

    #[test]
    fn markdown_agent_description_only_uses_defaults() {
        let content = "---\ndescription: Simple agent\n---\n";
        let agent = parse_markdown_agent("simple", content).unwrap();
        assert_eq!(agent.description, "Simple agent");
        assert!(agent.tools.is_empty());
        assert!(agent.skills.is_empty());
        assert!(agent.max_actions.is_none());
        assert!(agent.system_prompt.is_none());
    }

    #[test]
    fn markdown_agent_missing_description_skips() {
        let content = "---\ntools:\n  - comment\n---\nSome prompt.\n";
        let err = parse_markdown_agent("bot", content).unwrap_err();
        assert!(err.contains("description"));
    }

    #[test]
    fn markdown_agent_malformed_yaml_skips() {
        let content = "---\n: invalid: yaml: {{\n---\nSome prompt.\n";
        let err = parse_markdown_agent("bot", content).unwrap_err();
        assert!(err.contains("YAML") || err.contains("yaml") || err.contains("frontmatter"));
    }

    #[test]
    fn markdown_agent_missing_agents_dir_returns_empty() {
        let logger = Logger::new(Path::new("/dev/null"), false);
        let result = load_markdown_agents(Path::new("/nonexistent/agents"), &logger);
        assert!(result.is_empty());
    }
}
