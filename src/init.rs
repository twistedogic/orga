use std::path::Path;

use inquire::{Password, Select, Text};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;

use crate::config::{AgentConfig, AppConfig, BoardConfig, LlmConfig, MemoryConfig, SkillsConfig, WorkspaceConfig};
use crate::error::OrgaError;

// ── Linear init helpers ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LinearInitUser {
    #[allow(dead_code)]
    id: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct LinearTeamItem {
    pub id: String,
    pub name: String,
}

fn linear_gql<T: for<'de> Deserialize<'de>>(api_key: &str, query: &str) -> Result<T, OrgaError> {
    #[derive(serde::Serialize)]
    struct Payload<'a> { query: &'a str }
    #[derive(Deserialize)]
    struct GqlError { message: String }
    #[derive(Deserialize)]
    struct GqlResponse { data: Option<serde_json::Value>, errors: Option<Vec<GqlError>> }

    let client = Client::new();
    let resp = client
        .post("https://api.linear.app/graphql")
        .header("Authorization", api_key)
        .header("Content-Type", "application/json")
        .json(&Payload { query })
        .send()?;

    let status = resp.status();
    let body = resp.text().unwrap_or_default();

    if status == StatusCode::UNAUTHORIZED || status == StatusCode::BAD_REQUEST {
        let msg = serde_json::from_str::<GqlResponse>(&body)
            .ok()
            .and_then(|r| r.errors)
            .and_then(|e| e.into_iter().next())
            .map(|e| e.message)
            .unwrap_or_else(|| "invalid Linear API key".into());
        return Err(OrgaError::Unauthorized(msg));
    }
    if status.is_client_error() || status.is_server_error() {
        return Err(OrgaError::BackendError(format!("Linear returned HTTP {status}")));
    }

    let parsed: GqlResponse = serde_json::from_str(&body).map_err(|e| OrgaError::BackendError(e.to_string()))?;
    if let Some(first) = parsed.errors.and_then(|e| e.into_iter().next()) {
        return Err(OrgaError::BackendError(first.message));
    }
    let data = parsed.data.ok_or_else(|| OrgaError::BackendError("Linear returned no data".into()))?;
    serde_json::from_value(data).map_err(|e| OrgaError::BackendError(e.to_string()))
}

fn fetch_linear_viewer(api_key: &str) -> Result<LinearInitUser, OrgaError> {
    #[derive(Deserialize)]
    struct Resp { viewer: LinearInitUser }
    let resp: Resp = linear_gql(api_key, "query { viewer { id displayName } }")?;
    Ok(resp.viewer)
}

fn fetch_linear_teams(api_key: &str) -> Result<Vec<LinearTeamItem>, OrgaError> {
    #[derive(Deserialize)]
    struct Nodes { nodes: Vec<LinearTeamItem> }
    #[derive(Deserialize)]
    struct Resp { teams: Nodes }
    let resp: Resp = linear_gql(api_key, "query { teams { nodes { id name } } }")?;
    Ok(resp.teams.nodes)
}

#[derive(Debug, Deserialize)]
struct TrelloMeResponse {
    id: String,
    username: String,
    #[serde(rename = "fullName")]
    full_name: String,
}

#[derive(Debug, Deserialize)]
pub struct TrelloBoardItem {
    pub id: String,
    pub name: String,
}

fn fetch_me(api_key: &str, token: &str) -> Result<TrelloMeResponse, OrgaError> {
    let client = Client::new();
    let resp = client
        .get("https://api.trello.com/1/members/me")
        .query(&[("key", api_key), ("token", token)])
        .send()?;

    match resp.status() {
        StatusCode::UNAUTHORIZED => {
            return Err(OrgaError::Unauthorized(
                "invalid Trello API key or token".into(),
            ))
        }
        s if s.is_client_error() || s.is_server_error() => {
            return Err(OrgaError::BackendError(format!(
                "Trello returned HTTP {s}"
            )))
        }
        _ => {}
    }

    resp.json::<TrelloMeResponse>()
        .map_err(|e| OrgaError::BackendError(e.to_string()))
}

fn fetch_boards(api_key: &str, token: &str) -> Result<Vec<TrelloBoardItem>, OrgaError> {
    let client = Client::new();
    let resp = client
        .get("https://api.trello.com/1/members/me/boards")
        .query(&[("key", api_key), ("token", token), ("filter", "open")])
        .send()?;

    match resp.status() {
        StatusCode::UNAUTHORIZED => {
            return Err(OrgaError::Unauthorized(
                "invalid Trello API key or token".into(),
            ))
        }
        s if s.is_client_error() || s.is_server_error() => {
            return Err(OrgaError::BackendError(format!(
                "Trello returned HTTP {s}"
            )))
        }
        _ => {}
    }

    resp.json::<Vec<TrelloBoardItem>>()
        .map_err(|e| OrgaError::BackendError(e.to_string()))
}

pub fn run_board_init(config_path: &Path) -> Result<(), OrgaError> {
    let existing = AppConfig::try_load(config_path);

    let default_backend = existing
        .as_ref()
        .map(|c| c.board.backend.as_str())
        .unwrap_or("trello")
        .to_string();
    let backends = vec!["trello", "linear"];
    let default_backend_idx = backends
        .iter()
        .position(|b| *b == default_backend.as_str())
        .unwrap_or(0);
    let backend = Select::new("Backend:", backends)
        .with_starting_cursor(default_backend_idx)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    match backend {
        "linear" => run_linear_init(config_path, existing.as_ref()),
        _ => run_trello_init(config_path, existing.as_ref()),
    }
}

fn run_trello_init(config_path: &Path, existing: Option<&AppConfig>) -> Result<(), OrgaError> {
    let default_name = existing
        .map(|c| c.agent.name.as_str())
        .unwrap_or("")
        .to_string();
    let default_api_key = existing
        .and_then(|c| c.trello.as_ref())
        .map(|t| t.api_key.as_str())
        .unwrap_or("")
        .to_string();
    let default_token = existing
        .and_then(|c| c.trello.as_ref())
        .map(|t| t.token.as_str())
        .unwrap_or("")
        .to_string();
    let existing_board_id = existing
        .and_then(|c| c.trello.as_ref())
        .map(|t| t.board_id.as_str())
        .unwrap_or("")
        .to_string();

    let agent_name = Text::new("Agent name:")
        .with_default(&default_name)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    let api_key = Text::new("Trello API key:")
        .with_default(&default_api_key)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    let token = if default_token.is_empty() {
        Password::new("Trello token:")
            .without_confirmation()
            .prompt()
            .map_err(|e| OrgaError::ConfigError(e.to_string()))?
    } else {
        Password::new("Trello token (leave blank to keep current):")
            .without_confirmation()
            .prompt()
            .map_err(|e| OrgaError::ConfigError(e.to_string()))
            .map(|t| if t.is_empty() { default_token.clone() } else { t })?
    };

    print!("Fetching your Trello profile... ");
    let me = fetch_me(&api_key, &token)?;
    println!("Authenticated as @{} ({})", me.username, me.full_name);

    print!("Fetching your boards... ");
    let boards = fetch_boards(&api_key, &token)?;
    if boards.is_empty() {
        return Err(OrgaError::ConfigError(
            "no open boards found for this account".into(),
        ));
    }
    println!("found {} board(s)", boards.len());

    let default_board_idx = boards
        .iter()
        .position(|b| b.id == existing_board_id)
        .unwrap_or(0);

    let board_names: Vec<&str> = boards.iter().map(|b| b.name.as_str()).collect();
    let selected_name = Select::new("Which board?", board_names)
        .with_starting_cursor(default_board_idx)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    let board_id = boards
        .iter()
        .find(|b| b.name == selected_name)
        .map(|b| b.id.as_str())
        .unwrap_or_default();

    let mut config = AppConfig::try_load(config_path).unwrap_or_else(|| AppConfig {
        agent: crate::config::AgentConfig { name: agent_name.clone() },
        board: crate::config::BoardConfig { backend: "trello".into() },
        trello: None,
        linear: None,
        memory: None,
        logging: None,
        llm: None,
        workflow: vec![],
        comment_compaction_threshold: None,
        skills: None,
        workspace: None,
        subagents: vec![],
    });
    config.agent.name = agent_name;
    config.board.backend = "trello".into();
    config.trello = Some(crate::config::TrelloConfig {
        api_key,
        token,
        member_id: me.id,
        board_id: board_id.to_string(),
    });
    config.linear = None;
    config.save(config_path)?;
    AppConfig::load(config_path)
        .map_err(|e| OrgaError::ConfigError(format!("written config failed validation: {e}")))?;

    println!("Config written to {}", config_path.display());
    Ok(())
}

fn run_linear_init(config_path: &Path, existing: Option<&AppConfig>) -> Result<(), OrgaError> {
    let default_name = existing
        .map(|c| c.agent.name.as_str())
        .unwrap_or("")
        .to_string();
    let default_api_key = existing
        .and_then(|c| c.linear.as_ref())
        .map(|l| l.api_key.as_str())
        .unwrap_or("")
        .to_string();
    let existing_team_id = existing
        .and_then(|c| c.linear.as_ref())
        .map(|l| l.team_id.as_str())
        .unwrap_or("")
        .to_string();

    let agent_name = Text::new("Agent name:")
        .with_default(&default_name)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    let api_key = if default_api_key.is_empty() {
        Password::new("Linear API key:")
            .without_confirmation()
            .prompt()
            .map_err(|e| OrgaError::ConfigError(e.to_string()))?
    } else {
        Password::new("Linear API key (leave blank to keep current):")
            .without_confirmation()
            .prompt()
            .map_err(|e| OrgaError::ConfigError(e.to_string()))
            .map(|k| if k.is_empty() { default_api_key.clone() } else { k })?
    };

    print!("Verifying Linear API key... ");
    let viewer = fetch_linear_viewer(&api_key)?;
    println!("Authenticated as {}", viewer.display_name);

    print!("Fetching your teams... ");
    let teams = fetch_linear_teams(&api_key)?;
    if teams.is_empty() {
        return Err(OrgaError::ConfigError(
            "no teams found for this Linear account".into(),
        ));
    }
    println!("found {} team(s)", teams.len());

    let default_team_idx = teams
        .iter()
        .position(|t| t.id == existing_team_id)
        .unwrap_or(0);

    let team_names: Vec<&str> = teams.iter().map(|t| t.name.as_str()).collect();
    let selected_name = Select::new("Which team?", team_names)
        .with_starting_cursor(default_team_idx)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    let team_id = teams
        .iter()
        .find(|t| t.name == selected_name)
        .map(|t| t.id.as_str())
        .unwrap_or_default();

    let mut config = AppConfig::try_load(config_path).unwrap_or_else(|| AppConfig {
        agent: crate::config::AgentConfig { name: agent_name.clone() },
        board: crate::config::BoardConfig { backend: "linear".into() },
        trello: None,
        linear: None,
        memory: None,
        logging: None,
        llm: None,
        workflow: vec![],
        comment_compaction_threshold: None,
        skills: None,
        workspace: None,
        subagents: vec![],
    });
    config.agent.name = agent_name;
    config.board.backend = "linear".into();
    config.linear = Some(crate::config::LinearConfig {
        api_key,
        team_id: team_id.to_string(),
    });
    config.trello = None;
    config.save(config_path)?;
    AppConfig::load(config_path)
        .map_err(|e| OrgaError::ConfigError(format!("written config failed validation: {e}")))?;

    println!("Config written to {}", config_path.display());
    Ok(())
}

pub fn run_agent_init(config_path: &Path) -> Result<(), OrgaError> {
    let existing = AppConfig::try_load(config_path);

    let existing_provider = existing
        .as_ref()
        .and_then(|c| c.llm.as_ref())
        .map(|l| l.provider.as_str())
        .unwrap_or("anthropic")
        .to_string();
    let providers = vec!["anthropic", "openai"];
    let default_provider_idx = providers
        .iter()
        .position(|p| *p == existing_provider.as_str())
        .unwrap_or(0);
    let provider = Select::new("LLM provider:", providers)
        .with_starting_cursor(default_provider_idx)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    let default_api_key = existing
        .as_ref()
        .and_then(|c| c.llm.as_ref())
        .map(|l| l.api_key.as_str())
        .unwrap_or("")
        .to_string();
    let api_key = if default_api_key.is_empty() {
        Password::new("LLM API key:")
            .without_confirmation()
            .prompt()
            .map_err(|e| OrgaError::ConfigError(e.to_string()))?
    } else {
        Password::new("LLM API key (leave blank to keep current):")
            .without_confirmation()
            .prompt()
            .map_err(|e| OrgaError::ConfigError(e.to_string()))
            .map(|k| if k.is_empty() { default_api_key.clone() } else { k })?
    };

    let provider_default_model = if provider == "openai" { "gpt-4o" } else { "claude-opus-4-5" };
    let existing_model = existing
        .as_ref()
        .and_then(|c| c.llm.as_ref())
        .map(|l| l.model.as_str())
        .unwrap_or(provider_default_model)
        .to_string();
    let model = Text::new("Model:")
        .with_default(&existing_model)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    let default_memory = existing
        .as_ref()
        .and_then(|c| c.memory.as_ref())
        .and_then(|m| m.path.as_deref())
        .unwrap_or("")
        .to_string();
    let memory_path = Text::new("Memory DB path (leave blank to skip):")
        .with_default(&default_memory)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    let default_workspace = existing
        .as_ref()
        .and_then(|c| c.workspace.as_ref())
        .map(|w| w.path.as_str())
        .unwrap_or("")
        .to_string();
    let workspace_path = Text::new("Workspace path (leave blank to skip):")
        .with_default(&default_workspace)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    let default_skills = existing
        .as_ref()
        .and_then(|c| c.skills.as_ref())
        .map(|s| s.path.as_str())
        .unwrap_or("")
        .to_string();
    let skills_path = Text::new("Skills path (leave blank to skip):")
        .with_default(&default_skills)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    let mut config = existing.unwrap_or_else(|| AppConfig {
        agent: AgentConfig { name: String::new() },
        board: BoardConfig { backend: "trello".into() },
        trello: None,
        linear: None,
        memory: None,
        logging: None,
        llm: None,
        workflow: vec![],
        comment_compaction_threshold: None,
        skills: None,
        workspace: None,
        subagents: vec![],
    });

    config.llm = Some(LlmConfig {
        provider: provider.to_string(),
        api_key,
        model,
        endpoint: config.llm.as_ref().and_then(|l| l.endpoint.clone()),
        poll_interval_secs: config.llm.as_ref().and_then(|l| l.poll_interval_secs),
        max_actions_per_ticket: config.llm.as_ref().and_then(|l| l.max_actions_per_ticket),
    });

    config.memory = if memory_path.is_empty() {
        config.memory
    } else {
        Some(MemoryConfig { path: Some(memory_path) })
    };

    config.workspace = if workspace_path.is_empty() {
        config.workspace
    } else {
        Some(WorkspaceConfig { path: workspace_path })
    };

    config.skills = if skills_path.is_empty() {
        config.skills
    } else {
        Some(SkillsConfig { path: skills_path })
    };

    config.save(config_path)?;
    AppConfig::load(config_path)
        .map_err(|e| OrgaError::ConfigError(format!("written config failed validation: {e}")))?;

    println!("Config written to {}", config_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentConfig, BoardConfig, LinearConfig, TrelloConfig};
    use tempfile::NamedTempFile;

    fn make_trello_config(name: &str, board_id: &str, api_key: &str, token: &str, member_id: &str) -> AppConfig {
        AppConfig {
            agent: AgentConfig { name: name.into() },
            board: BoardConfig { backend: "trello".into() },
            trello: Some(TrelloConfig {
                api_key: api_key.into(),
                token: token.into(),
                member_id: member_id.into(),
                board_id: board_id.into(),
            }),
            linear: None,
            memory: None,
            logging: None,
            llm: None,
            workflow: vec![],
            comment_compaction_threshold: None,
            skills: None,
            workspace: None,
            subagents: vec![],
        }
    }

    fn make_linear_config(name: &str, team_id: &str, api_key: &str) -> AppConfig {
        AppConfig {
            agent: AgentConfig { name: name.into() },
            board: BoardConfig { backend: "linear".into() },
            trello: None,
            linear: Some(LinearConfig { api_key: api_key.into(), team_id: team_id.into() }),
            memory: None,
            logging: None,
            llm: None,
            workflow: vec![],
            comment_compaction_threshold: None,
            skills: None,
            workspace: None,
            subagents: vec![],
        }
    }

    #[test]
    fn write_config_file_produces_valid_toml() {
        let f = NamedTempFile::new().unwrap();
        let cfg = make_trello_config("agent-1", "board-abc", "key123", "tok456", "mem789");
        cfg.save(f.path()).unwrap();
        let loaded = AppConfig::load(f.path()).unwrap();
        assert_eq!(loaded.agent.name, "agent-1");
        assert_eq!(loaded.trello.as_ref().unwrap().board_id, "board-abc");
        assert_eq!(loaded.board.backend, "trello");
        let trello = loaded.trello.unwrap();
        assert_eq!(trello.api_key, "key123");
        assert_eq!(trello.token, "tok456");
        assert_eq!(trello.member_id, "mem789");
    }

    #[test]
    fn write_config_file_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("config.toml");
        let cfg = make_trello_config("agent-x", "board-x", "k", "t", "m");
        cfg.save(&nested).unwrap();
        assert!(nested.exists());
    }

    #[test]
    fn write_config_file_overwrites_existing_preserving_new_values() {
        let f = NamedTempFile::new().unwrap();
        make_trello_config("old-name", "old-board", "old-key", "old-tok", "old-mem")
            .save(f.path()).unwrap();
        make_trello_config("new-name", "new-board", "new-key", "new-tok", "new-mem")
            .save(f.path()).unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.agent.name, "new-name");
        assert_eq!(cfg.trello.as_ref().unwrap().board_id, "new-board");
    }

    #[test]
    fn write_linear_config_file_produces_valid_toml() {
        let f = NamedTempFile::new().unwrap();
        make_linear_config("agent-1", "team-abc", "lin_api_xyz").save(f.path()).unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.agent.name, "agent-1");
        assert_eq!(cfg.board.backend, "linear");
        assert_eq!(cfg.linear.as_ref().unwrap().team_id, "team-abc");
        assert_eq!(cfg.linear.unwrap().api_key, "lin_api_xyz");
        assert!(cfg.trello.is_none());
    }

}
