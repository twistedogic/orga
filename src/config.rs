use std::path::{Path, PathBuf};
use std::{env, fs};

use serde::Deserialize;

use crate::error::OrgaError;

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct BoardConfig {
    pub id: String,
    pub backend: String,
}

#[derive(Debug, Deserialize)]
pub struct TrelloConfig {
    pub api_key: String,
    pub token: String,
    pub member_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MemoryConfig {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub agent: AgentConfig,
    pub board: BoardConfig,
    pub trello: Option<TrelloConfig>,
    pub memory: Option<MemoryConfig>,
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, OrgaError> {
        let content = fs::read_to_string(path).map_err(|e| {
            OrgaError::ConfigError(format!(
                "cannot read config at {}: {}",
                path.display(),
                e
            ))
        })?;
        let config: AppConfig = toml::from_str(&content)
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

    fn validate(&self) -> Result<(), OrgaError> {
        const SUPPORTED: &[&str] = &["trello"];
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
        Ok(())
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
id = "board-xyz"
backend = "trello"

[trello]
api_key = "key"
token = "tok"
member_id = "abc123"
"#;

    #[test]
    fn valid_config_loads() {
        let f = write_config(VALID_CONFIG);
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.trello.unwrap().member_id, "abc123");
        assert_eq!(cfg.board.backend, "trello");
    }

    #[test]
    fn unknown_backend_fails() {
        let content = VALID_CONFIG.replace("backend = \"trello\"", "backend = \"linear\"");
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
id = "board-xyz"
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
}
