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
pub struct ArtifactGitConfig {
    pub path: String,
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub ssh_key: Option<String>,
    pub ssh_passphrase: Option<String>,
    pub http_username: Option<String>,
    pub http_password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowEntry {
    pub column: String,
    pub prompt: Option<String>,
    pub prompt_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub file: Option<String>,
    pub debug: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ArtifactConfig {
    pub backend: String,
    pub git: Option<ArtifactGitConfig>,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub agent: AgentConfig,
    pub board: BoardConfig,
    pub trello: Option<TrelloConfig>,
    pub linear: Option<LinearConfig>,
    pub memory: Option<MemoryConfig>,
    pub artifact: Option<ArtifactConfig>,
    pub logging: Option<LoggingConfig>,
    #[serde(default)]
    pub workflow: Vec<WorkflowEntry>,
    pub comment_compaction_threshold: Option<usize>,
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
        if self.board.backend == "trello" {
            if let Some(ref t) = self.trello {
                if t.board_id.is_empty() {
                    return Err(OrgaError::ConfigError(
                        "[trello] board_id is required".into(),
                    ));
                }
            }
        }
        if self.board.backend == "linear" {
            if let Some(ref l) = self.linear {
                if l.team_id.is_empty() {
                    return Err(OrgaError::ConfigError(
                        "[linear] team_id is required".into(),
                    ));
                }
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
    fn artifact_config_absent() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.artifact.is_none());
    }

    #[test]
    fn artifact_config_git_section() {
        let content = format!(
            "{VALID_CONFIG}\n[artifact]\nbackend = \"git\"\n\n[artifact.git]\npath = \"/tmp/artifacts\"\n"
        );
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        let artifact = cfg.artifact.unwrap();
        assert_eq!(artifact.backend, "git");
        let git = artifact.git.unwrap();
        assert_eq!(git.path, "/tmp/artifacts");
        assert!(git.remote.is_none());
        assert!(git.branch.is_none());
    }

    #[test]
    fn artifact_config_git_with_remote_and_branch() {
        let content = format!(
            "{VALID_CONFIG}\n[artifact]\nbackend = \"git\"\n\n[artifact.git]\npath = \"/tmp/artifacts\"\nremote = \"origin\"\nbranch = \"main\"\n"
        );
        let f = write_config(&content);
        let cfg = AppConfig::load(f.path()).unwrap();
        let git = cfg.artifact.unwrap().git.unwrap();
        assert_eq!(git.remote.as_deref(), Some("origin"));
        assert_eq!(git.branch.as_deref(), Some("main"));
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
}
