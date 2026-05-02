use std::fs;
use std::path::Path;

use inquire::{Password, Select, Text};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;

use crate::config::AppConfig;
use crate::error::OrgaError;

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

pub fn run_init(config_path: &Path) -> Result<(), OrgaError> {
    let existing = AppConfig::try_load(config_path);

    let default_name = existing
        .as_ref()
        .map(|c| c.agent.name.as_str())
        .unwrap_or("")
        .to_string();
    let default_api_key = existing
        .as_ref()
        .and_then(|c| c.trello.as_ref())
        .map(|t| t.api_key.as_str())
        .unwrap_or("")
        .to_string();
    let default_token = existing
        .as_ref()
        .and_then(|c| c.trello.as_ref())
        .map(|t| t.token.as_str())
        .unwrap_or("")
        .to_string();
    let existing_board_id = existing
        .as_ref()
        .map(|c| c.board.id.as_str())
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

    write_config_file(
        config_path,
        &agent_name,
        board_id,
        &api_key,
        &token,
        &me.id,
    )?;

    println!("Config written to {}", config_path.display());
    Ok(())
}

fn write_config_file(
    config_path: &Path,
    agent_name: &str,
    board_id: &str,
    api_key: &str,
    token: &str,
    member_id: &str,
) -> Result<(), OrgaError> {
    let toml = format!(
        "[agent]\nname = {agent_name:?}\n\n[board]\nid = {board_id:?}\nbackend = \"trello\"\n\n[trello]\napi_key = {api_key:?}\ntoken = {token:?}\nmember_id = {member_id:?}\n",
    );

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            OrgaError::ConfigError(format!(
                "cannot create config directory {}: {e}",
                parent.display()
            ))
        })?;
    }

    fs::write(config_path, &toml).map_err(|e| {
        OrgaError::ConfigError(format!(
            "cannot write config to {}: {e}",
            config_path.display()
        ))
    })?;

    AppConfig::load(config_path).map_err(|e| {
        OrgaError::ConfigError(format!("written config failed validation: {e}"))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn write_config_file_produces_valid_toml() {
        let f = NamedTempFile::new().unwrap();
        write_config_file(f.path(), "agent-1", "board-abc", "key123", "tok456", "mem789").unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.agent.name, "agent-1");
        assert_eq!(cfg.board.id, "board-abc");
        assert_eq!(cfg.board.backend, "trello");
        let trello = cfg.trello.unwrap();
        assert_eq!(trello.api_key, "key123");
        assert_eq!(trello.token, "tok456");
        assert_eq!(trello.member_id, "mem789");
    }

    #[test]
    fn write_config_file_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("config.toml");
        write_config_file(&nested, "agent-x", "board-x", "k", "t", "m").unwrap();
        assert!(nested.exists());
    }

    #[test]
    fn write_config_file_overwrites_existing_preserving_new_values() {
        let f = NamedTempFile::new().unwrap();
        write_config_file(f.path(), "old-name", "old-board", "old-key", "old-tok", "old-mem")
            .unwrap();
        write_config_file(f.path(), "new-name", "new-board", "new-key", "new-tok", "new-mem")
            .unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.agent.name, "new-name");
        assert_eq!(cfg.board.id, "new-board");
    }
}
