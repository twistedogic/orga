use std::fs;
use std::path::{Path, PathBuf};

type RepoTriple = Result<(Option<String>, Option<String>, Option<String>), OrgaError>;

use git2::Repository;
use inquire::{Confirm, Password, Select, Text};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;

use crate::config::{AppConfig, ArtifactGitConfig};
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

pub fn run_init(config_path: &Path) -> Result<(), OrgaError> {
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

    let artifact = run_artifact_setup(existing.and_then(|c| c.artifact.as_ref()).and_then(|a| a.git.as_ref()))?;

    write_config_file(
        config_path,
        &agent_name,
        board_id,
        &api_key,
        &token,
        &me.id,
        artifact.as_ref(),
    )?;

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

    let artifact = run_artifact_setup(existing.and_then(|c| c.artifact.as_ref()).and_then(|a| a.git.as_ref()))?;

    write_linear_config_file(config_path, &agent_name, team_id, &api_key, artifact.as_ref())?;

    println!("Config written to {}", config_path.display());
    Ok(())
}

fn run_artifact_setup(existing: Option<&ArtifactGitConfig>) -> Result<Option<ArtifactGitConfig>, OrgaError> {
    let configure = Confirm::new("Configure artifact store?")
        .with_default(existing.is_some())
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    if !configure {
        return Ok(None);
    }

    let default_path = existing
        .map(|g| g.path.as_str())
        .unwrap_or("~/.orga/artifacts")
        .to_string();

    let path_str = Text::new("Artifact store path:")
        .with_default(&default_path)
        .prompt()
        .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

    let expanded = crate::config::expand_tilde(&path_str);

    let (remote, branch, ssh_key) = detect_or_setup_repo(&expanded, existing)?;

    Ok(Some(ArtifactGitConfig {
        path: path_str,
        remote,
        branch,
        ssh_key,
        ssh_passphrase: None,
        http_username: None,
        http_password: None,
    }))
}

fn detect_or_setup_repo(
    path: &PathBuf,
    existing: Option<&ArtifactGitConfig>,
) -> RepoTriple {
    if path.exists() {
        open_existing_repo(path, existing)
    } else {
        // Path doesn't exist — ask for optional remote URL
        let url = Text::new("Remote URL (leave blank for local-only):")
            .with_default("")
            .prompt()
            .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

        if url.is_empty() {
            init_local_repo(path)?;
            return Ok((None, None, None));
        }

        let default_branch = existing
            .and_then(|g| g.branch.as_deref())
            .unwrap_or("main")
            .to_string();
        let default_remote = existing
            .and_then(|g| g.remote.as_deref())
            .unwrap_or("origin")
            .to_string();
        let default_ssh_key = existing
            .and_then(|g| g.ssh_key.as_deref())
            .unwrap_or("~/.ssh/id_rsa")
            .to_string();

        let branch = Text::new("Branch:")
            .with_default(&default_branch)
            .prompt()
            .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

        let remote_name = Text::new("Remote name:")
            .with_default(&default_remote)
            .prompt()
            .map_err(|e| OrgaError::ConfigError(e.to_string()))?;

        let ssh_key_input = Text::new("SSH key path (leave blank to use SSH agent):")
            .with_default(&default_ssh_key)
            .prompt()
            .map_err(|e| OrgaError::ConfigError(e.to_string()))?;
        let ssh_key = if ssh_key_input.is_empty() { None } else { Some(ssh_key_input) };

        print!("Cloning {}... ", url);
        clone_with_ssh_key_or_agent(&url, path, ssh_key.as_deref()).map_err(|e| {
            OrgaError::ConfigError(format!(
                "clone failed: {e}\nMake sure your SSH key is correct or your SSH agent is running."
            ))
        })?;
        println!("done");

        Ok((Some(remote_name), Some(branch), ssh_key))
    }
}

fn open_existing_repo(
    path: &PathBuf,
    existing: Option<&ArtifactGitConfig>,
) -> RepoTriple {
    Repository::open(path).map_err(|_| {
        OrgaError::ConfigError(format!(
            "path '{}' exists but is not a valid git repository",
            path.display()
        ))
    })?;
    let remote = existing.and_then(|g| g.remote.clone());
    let branch = existing.and_then(|g| g.branch.clone());
    let ssh_key = existing.and_then(|g| g.ssh_key.clone());
    Ok((remote, branch, ssh_key))
}

fn init_local_repo(path: &PathBuf) -> Result<(), OrgaError> {
    Repository::init(path).map_err(|e| {
        OrgaError::ConfigError(format!("git init failed: {e}"))
    })?;
    println!("Initialized empty git repository at {}", path.display());
    Ok(())
}

fn clone_with_ssh_key_or_agent(url: &str, into: &Path, ssh_key: Option<&str>) -> Result<(), git2::Error> {
    let key_path = ssh_key.map(crate::config::expand_tilde);
    let mut tried = false;
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(move |_url, username_from_url, _allowed| {
        if tried {
            return Err(git2::Error::from_str("authentication failed"));
        }
        tried = true;
        let username = username_from_url.unwrap_or("git");
        match &key_path {
            Some(kp) => git2::Cred::ssh_key(username, None, kp, None),
            None => git2::Cred::ssh_key_from_agent(username),
        }
    });

    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_opts);
    builder.clone(url, into)?;
    Ok(())
}

fn write_linear_config_file(
    config_path: &Path,
    agent_name: &str,
    team_id: &str,
    api_key: &str,
    artifact: Option<&ArtifactGitConfig>,
) -> Result<(), OrgaError> {
    let mut toml = format!(
        "[agent]\nname = {agent_name:?}\n\n[board]\nbackend = \"linear\"\n\n[linear]\napi_key = {api_key:?}\nteam_id = {team_id:?}\n",
    );

    if let Some(git) = artifact {
        toml.push_str("\n[artifact]\nbackend = \"git\"\n\n[artifact.git]\n");
        toml.push_str(&format!("path = {:?}\n", git.path));
        if let Some(ref remote) = git.remote {
            toml.push_str(&format!("remote = {remote:?}\n"));
        }
        if let Some(ref branch) = git.branch {
            toml.push_str(&format!("branch = {branch:?}\n"));
        }
        if let Some(ref ssh_key) = git.ssh_key {
            toml.push_str(&format!("ssh_key = {ssh_key:?}\n"));
        }
    }

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

fn write_config_file(
    config_path: &Path,
    agent_name: &str,
    board_id: &str,
    api_key: &str,
    token: &str,
    member_id: &str,
    artifact: Option<&ArtifactGitConfig>,
) -> Result<(), OrgaError> {
    let mut toml = format!(
        "[agent]\nname = {agent_name:?}\n\n[board]\nbackend = \"trello\"\n\n[trello]\napi_key = {api_key:?}\ntoken = {token:?}\nmember_id = {member_id:?}\nboard_id = {board_id:?}\n",
    );

    if let Some(git) = artifact {
        toml.push_str("\n[artifact]\nbackend = \"git\"\n\n[artifact.git]\n");
        toml.push_str(&format!("path = {:?}\n", git.path));
        if let Some(ref remote) = git.remote {
            toml.push_str(&format!("remote = {remote:?}\n"));
        }
        if let Some(ref branch) = git.branch {
            toml.push_str(&format!("branch = {branch:?}\n"));
        }
        if let Some(ref ssh_key) = git.ssh_key {
            toml.push_str(&format!("ssh_key = {ssh_key:?}\n"));
        }
    }

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
        write_config_file(f.path(), "agent-1", "board-abc", "key123", "tok456", "mem789", None).unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.agent.name, "agent-1");
        assert_eq!(cfg.trello.as_ref().unwrap().board_id, "board-abc");
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
        write_config_file(&nested, "agent-x", "board-x", "k", "t", "m", None).unwrap();
        assert!(nested.exists());
    }

    #[test]
    fn write_config_file_overwrites_existing_preserving_new_values() {
        let f = NamedTempFile::new().unwrap();
        write_config_file(f.path(), "old-name", "old-board", "old-key", "old-tok", "old-mem", None)
            .unwrap();
        write_config_file(f.path(), "new-name", "new-board", "new-key", "new-tok", "new-mem", None)
            .unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.agent.name, "new-name");
        assert_eq!(cfg.trello.as_ref().unwrap().board_id, "new-board");
    }

    #[test]
    fn write_config_file_no_artifact_section() {
        let f = NamedTempFile::new().unwrap();
        write_config_file(f.path(), "agent-1", "board-abc", "key", "tok", "mem", None).unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.artifact.is_none());
    }

    #[test]
    fn write_config_file_local_artifact_section() {
        let f = NamedTempFile::new().unwrap();
        let git_cfg = ArtifactGitConfig {
            path: "/tmp/artifacts".to_string(),
            remote: None,
            branch: None,
            ssh_key: None,
            ssh_passphrase: None,
            http_username: None,
            http_password: None,
        };
        write_config_file(f.path(), "agent-1", "board-abc", "key", "tok", "mem", Some(&git_cfg))
            .unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        let artifact = cfg.artifact.unwrap();
        assert_eq!(artifact.backend, "git");
        let git = artifact.git.unwrap();
        assert_eq!(git.path, "/tmp/artifacts");
        assert!(git.remote.is_none());
        assert!(git.branch.is_none());
    }

    #[test]
    fn write_config_file_artifact_with_remote_and_branch() {
        let f = NamedTempFile::new().unwrap();
        let git_cfg = ArtifactGitConfig {
            path: "/tmp/artifacts".to_string(),
            remote: Some("origin".to_string()),
            branch: Some("main".to_string()),
            ssh_key: None,
            ssh_passphrase: None,
            http_username: None,
            http_password: None,
        };
        write_config_file(f.path(), "agent-1", "board-abc", "key", "tok", "mem", Some(&git_cfg))
            .unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        let git = cfg.artifact.unwrap().git.unwrap();
        assert_eq!(git.path, "/tmp/artifacts");
        assert_eq!(git.remote.as_deref(), Some("origin"));
        assert_eq!(git.branch.as_deref(), Some("main"));
    }

    #[test]
    fn write_config_file_artifact_with_ssh_key() {
        let f = NamedTempFile::new().unwrap();
        let git_cfg = ArtifactGitConfig {
            path: "/tmp/artifacts".to_string(),
            remote: Some("origin".to_string()),
            branch: Some("main".to_string()),
            ssh_key: Some("~/.ssh/id_ed25519".to_string()),
            ssh_passphrase: None,
            http_username: None,
            http_password: None,
        };
        write_config_file(f.path(), "agent-1", "board-abc", "key", "tok", "mem", Some(&git_cfg))
            .unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        let git = cfg.artifact.unwrap().git.unwrap();
        assert_eq!(git.ssh_key.as_deref(), Some("~/.ssh/id_ed25519"));
    }

    // --- 3.1: skipped artifact setup writes no artifact sections ---
    #[test]
    fn write_config_file_skipped_artifact_writes_no_artifact_section() {
        let f = NamedTempFile::new().unwrap();
        write_config_file(f.path(), "agent-1", "board-abc", "key", "tok", "mem", None).unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(!content.contains("[artifact]"));
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.artifact.is_none());
    }

    // --- 3.2: local-init path: missing dir is created, repo opens, config written with path only ---
    #[test]
    fn init_local_repo_creates_valid_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().join("artifacts");
        assert!(!repo_path.exists());
        init_local_repo(&repo_path).unwrap();
        assert!(repo_path.exists());
        Repository::open(&repo_path).expect("should be a valid git repo");

        let f = NamedTempFile::new().unwrap();
        let git_cfg = ArtifactGitConfig {
            path: repo_path.to_str().unwrap().to_string(),
            remote: None,
            branch: None,
            ssh_key: None,
            ssh_passphrase: None,
            http_username: None,
            http_password: None,
        };
        write_config_file(f.path(), "agent-1", "board-abc", "key", "tok", "mem", Some(&git_cfg))
            .unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        let git = cfg.artifact.unwrap().git.unwrap();
        assert!(git.remote.is_none());
        assert!(git.branch.is_none());
    }

    // --- 3.3: existing repo path accepted, config written with path only (no remote) ---
    #[test]
    fn open_existing_repo_accepts_valid_repo() {
        let dir = tempfile::tempdir().unwrap();
        Repository::init(dir.path()).unwrap();
        let (remote, branch, ssh_key) = open_existing_repo(&dir.path().to_path_buf(), None).unwrap();
        assert!(remote.is_none());
        assert!(branch.is_none());
        assert!(ssh_key.is_none());
    }

    #[test]
    fn open_existing_repo_carries_over_remote_and_branch() {
        let dir = tempfile::tempdir().unwrap();
        Repository::init(dir.path()).unwrap();
        let existing = ArtifactGitConfig {
            path: dir.path().to_str().unwrap().to_string(),
            remote: Some("origin".to_string()),
            branch: Some("main".to_string()),
            ssh_key: None,
            ssh_passphrase: None,
            http_username: None,
            http_password: None,
        };
        let (remote, branch, ssh_key) =
            open_existing_repo(&dir.path().to_path_buf(), Some(&existing)).unwrap();
        assert_eq!(remote.as_deref(), Some("origin"));
        assert_eq!(branch.as_deref(), Some("main"));
        assert!(ssh_key.is_none());
    }

    #[test]
    fn write_linear_config_file_produces_valid_toml() {
        let f = NamedTempFile::new().unwrap();
        write_linear_config_file(f.path(), "agent-1", "team-abc", "lin_api_xyz", None).unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        assert_eq!(cfg.agent.name, "agent-1");
        assert_eq!(cfg.board.backend, "linear");
        assert_eq!(cfg.linear.as_ref().unwrap().team_id, "team-abc");
        assert_eq!(cfg.linear.unwrap().api_key, "lin_api_xyz");
        assert!(cfg.trello.is_none());
    }

    #[test]
    fn write_linear_config_file_no_artifact_section() {
        let f = NamedTempFile::new().unwrap();
        write_linear_config_file(f.path(), "agent-1", "team-abc", "lin_api_xyz", None).unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        assert!(cfg.artifact.is_none());
    }

    #[test]
    fn write_linear_config_file_with_artifact() {
        let f = NamedTempFile::new().unwrap();
        let git_cfg = ArtifactGitConfig {
            path: "/tmp/artifacts".to_string(),
            remote: Some("origin".to_string()),
            branch: Some("main".to_string()),
            ssh_key: None,
            ssh_passphrase: None,
            http_username: None,
            http_password: None,
        };
        write_linear_config_file(f.path(), "agent-1", "team-abc", "lin_api_xyz", Some(&git_cfg)).unwrap();
        let cfg = AppConfig::load(f.path()).unwrap();
        let git = cfg.artifact.unwrap().git.unwrap();
        assert_eq!(git.path, "/tmp/artifacts");
        assert_eq!(git.remote.as_deref(), Some("origin"));
        assert_eq!(git.branch.as_deref(), Some("main"));
    }
}
