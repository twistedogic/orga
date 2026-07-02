use std::path::{Path, PathBuf};
use std::{env, fs};

use serde::{Deserialize, Serialize};

use crate::error::OrgaError;
use crate::logging::Logger;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoardConfig {
    pub backend: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinearConfig {
    pub api_key: String,
    pub team_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrelloConfig {
    pub api_key: String,
    pub token: String,
    pub member_id: String,
    pub board_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemoryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defrag_file_threshold: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defrag_size_threshold_kb: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkspaceConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillsConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubagentConfig {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_actions: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsConfig {
    #[serde(
        default = "default_metrics_listen_addr",
        skip_serializing_if = "Option::is_none"
    )]
    pub listen_addr: Option<String>,
}

fn default_metrics_listen_addr() -> Option<String> {
    Some("127.0.0.1:9090".to_string())
}

impl MetricsConfig {
    pub fn listen_addr(&self) -> &str {
        self.listen_addr.as_deref().unwrap_or("127.0.0.1:9090")
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_interval_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub agent: AgentConfig,
    pub board: BoardConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trello: Option<TrelloConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear: Option<LinearConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<MetricsConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_compaction_threshold: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<SkillsConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subagents: Vec<SubagentConfig>,
}

impl AppConfig {
    pub fn try_load(path: &Path) -> Option<Self> {
        Self::load(path).ok()
    }

    pub fn load(path: &Path) -> Result<Self, OrgaError> {
        let content = fs::read_to_string(path).map_err(|e| {
            OrgaError::ConfigError(format!("cannot read config at {}: {}", path.display(), e))
        })?;
        let mut config: AppConfig = toml::from_str(&content)
            .map_err(|e| OrgaError::ConfigError(format!("invalid config: {e}")))?;
        if let Some(parent) = path.parent() {
            let agents_dir = parent.join("agents");
            let logger = config.logger();
            let md_agents = crate::agent::agents::load_markdown_agents(&agents_dir, &logger);
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

    pub fn memory_repo_path(&self) -> PathBuf {
        let raw = self
            .memory
            .as_ref()
            .and_then(|m| m.path.as_deref())
            .unwrap_or("~/.orga/memory");
        expand_tilde(raw)
    }

    pub fn defrag_file_threshold(&self) -> usize {
        self.memory
            .as_ref()
            .and_then(|m| m.defrag_file_threshold)
            .unwrap_or(20)
    }

    pub fn defrag_size_threshold_kb(&self) -> u64 {
        self.memory
            .as_ref()
            .and_then(|m| m.defrag_size_threshold_kb)
            .unwrap_or(50)
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
            && t.board_id.is_empty()
        {
            return Err(OrgaError::ConfigError(
                "[trello] board_id is required".into(),
            ));
        }
        if self.board.backend == "linear"
            && let Some(ref l) = self.linear
            && l.team_id.is_empty()
        {
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
                return Err(OrgaError::ConfigError("[llm] api_key is required".into()));
            }
            if llm.model.is_empty() {
                return Err(OrgaError::ConfigError("[llm] model is required".into()));
            }
        }
        if let Some(ref m) = self.metrics {
            let addr = m.listen_addr();
            if addr.parse::<std::net::SocketAddr>().is_err() {
                return Err(OrgaError::ConfigError(format!(
                    "[metrics] listen_addr '{}' is not a valid host:port",
                    addr
                )));
            }
        }
        // Validate subagents
        const VALID_TOOLS: &[&str] = &[
            "comment",
            "assign",
            "create_sub",
            "compact",
            "done",
            "skip",
            "dispatch",
            "return",
            "bash",
            "todos",
            "memory_list",
            "memory_read",
            "memory_write",
            "memory_search",
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
        self.make_logger(false)
    }

    pub fn agent_logger(&self) -> Logger {
        self.make_logger(true)
    }

    fn make_logger(&self, stdout: bool) -> Logger {
        let path = self
            .logging
            .as_ref()
            .and_then(|l| l.file.as_deref())
            .unwrap_or("~/.orga/orga.log");
        let debug = self.logging.as_ref().and_then(|l| l.debug).unwrap_or(false);
        Logger::with_stdout(&expand_tilde(path), debug, stdout)
    }

    pub fn agents_md_path(&self) -> Option<PathBuf> {
        self.workspace_base_path().map(|p| p.join("AGENTS.md"))
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

    pub fn metrics_config(&self) -> Option<&MetricsConfig> {
        self.metrics.as_ref()
    }

    pub fn save(&self, path: &Path) -> Result<(), OrgaError> {
        let toml = toml::to_string(self)
            .map_err(|e| OrgaError::ConfigError(format!("failed to serialize config: {e}")))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                OrgaError::ConfigError(format!(
                    "cannot create config directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        fs::write(path, &toml).map_err(|e| {
            OrgaError::ConfigError(format!("cannot write config to {}: {e}", path.display()))
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
        assert_eq!(
            cfg.logging.as_ref().unwrap().file.as_deref(),
            Some("/tmp/orga-test.log")
        );
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
    fn metrics_section_absent_uses_none() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.metrics.is_none());
    }

    #[test]
    fn metrics_section_default_listen_addr() {
        let content = format!("{VALID_CONFIG}\n[metrics]\n");
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(
            cfg.metrics_config().unwrap().listen_addr(),
            "127.0.0.1:9090"
        );
    }

    #[test]
    fn metrics_section_custom_listen_addr() {
        let content = format!("{VALID_CONFIG}\n[metrics]\nlisten_addr = \"0.0.0.0:9100\"\n");
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.metrics_config().unwrap().listen_addr(), "0.0.0.0:9100");
    }

    #[test]
    fn metrics_invalid_listen_addr_rejected() {
        let content = format!("{VALID_CONFIG}\n[metrics]\nlisten_addr = \"not-a-socket-addr\"\n");
        let f = write_config(&content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("listen_addr"));
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
        let content =
            VALID_LLM_CONFIG.replace("provider = \"anthropic\"", "provider = \"unknown\"");
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
        assert_eq!(
            cfg.llm.as_ref().unwrap().endpoint.as_deref(),
            Some("https://proxy.example.com/v1")
        );
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
        let content = format!(
            "{VALID_CONFIG}\n[[subagents]]\nname = \"researcher\"\ndescription = \"Does research\"\ntools = [\"comment\", \"done\"]\n"
        );
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.subagents.len(), 1);
        assert_eq!(cfg.subagents[0].name, "researcher");
        assert_eq!(cfg.subagents[0].tools, vec!["comment", "done"]);
    }

    #[test]
    fn subagent_config_with_optional_fields() {
        let content = format!(
            "{VALID_CONFIG}\n[[subagents]]\nname = \"drafter\"\ndescription = \"Drafts content\"\ntools = [\"comment\"]\nskills = [\"writing\"]\nmodel = \"gpt-4o\"\nmax_actions = 20\n"
        );
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.subagents[0].model.as_deref(), Some("gpt-4o"));
        assert_eq!(cfg.subagents[0].max_actions, Some(20));
        assert_eq!(cfg.subagents[0].skills, vec!["writing"]);
    }

    #[test]
    fn duplicate_subagent_name_fails_validation() {
        let content = format!(
            "{VALID_CONFIG}\n[[subagents]]\nname = \"bot\"\ndescription = \"a\"\ntools = [\"done\"]\n[[subagents]]\nname = \"bot\"\ndescription = \"b\"\ntools = [\"done\"]\n"
        );
        let f = write_config(&content);
        let err = AppConfig::load(f.path()).unwrap_err();
        assert!(err.to_string().contains("duplicate subagent name"));
    }

    #[test]
    fn unknown_tool_name_in_subagent_fails_validation() {
        let content = format!(
            "{VALID_CONFIG}\n[[subagents]]\nname = \"bot\"\ndescription = \"a\"\ntools = [\"fly\"]\n"
        );
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

    // --- serialization / save tests ---

    #[test]
    fn config_round_trips_through_serialize_deserialize() {
        let f = write_config(VALID_LLM_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        let toml_str = toml::to_string(&cfg).unwrap();
        let cfg2: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg2.board.backend, cfg.board.backend);
        assert_eq!(cfg2.agent.name, cfg.agent.name);
        assert_eq!(cfg2.llm.as_ref().unwrap().provider, "anthropic");
    }

    #[test]
    fn none_fields_omitted_from_serialized_output() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.linear.is_none());
        let toml_str = toml::to_string(&cfg).unwrap();
        assert!(!toml_str.contains("[linear]"));
        assert!(!toml_str.contains("[llm]"));
        assert!(!toml_str.contains("[memory]"));
    }

    #[test]
    fn empty_vec_fields_omitted_from_serialized_output() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.subagents.is_empty());
        let toml_str = toml::to_string(&cfg).unwrap();
        assert!(!toml_str.contains("[[subagents]]"));
    }

    #[test]
    fn save_writes_valid_toml_and_reloads() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let f = write_config(VALID_LLM_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        cfg.save(&path).unwrap();
        let cfg2 = AppConfig::load(&path).unwrap();
        assert_eq!(cfg2.agent.name, cfg.agent.name);
        assert_eq!(cfg2.llm.as_ref().unwrap().model, "claude-opus-4-5");
    }

    #[test]
    fn save_creates_parent_directories() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("deep").join("config.toml");
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        cfg.save(&path).unwrap();
        assert!(path.exists());
    }
}
