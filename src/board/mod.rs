use crate::config::AppConfig;
use crate::error::OrgaError;
use crate::models::{Column, Member, Ticket};

pub mod trello;

pub trait Board {
    fn list_assigned(&self) -> Result<Vec<Ticket>, OrgaError>;
    fn get_ticket(&self, id: &str) -> Result<Ticket, OrgaError>;
    fn comment(&self, id: &str, text: &str) -> Result<(), OrgaError>;
    fn assign(&self, id: &str, username: &str) -> Result<(), OrgaError>;
    fn move_ticket(&self, id: &str, list: &str) -> Result<(), OrgaError>;
    fn create_sub(&self, parent_id: &str, title: &str) -> Result<Ticket, OrgaError>;
    fn add_checklist_item(&self, id: &str, text: &str) -> Result<String, OrgaError>;
    fn check_item(&self, id: &str, item_id: &str) -> Result<(), OrgaError>;
    fn list_columns(&self) -> Result<Vec<Column>, OrgaError>;
    fn whoami(&self) -> Result<Member, OrgaError>;
    fn return_ticket(&self, id: &str, comment: Option<&str>) -> Result<(), OrgaError>;
}

pub fn build_board(config: &AppConfig) -> Result<Box<dyn Board>, OrgaError> {
    match config.board.backend.as_str() {
        "trello" => {
            let trello_cfg = config.trello.as_ref().ok_or_else(|| {
                OrgaError::ConfigError("[trello] section missing".into())
            })?;
            let backend = trello::TrelloBackend::new(
                trello_cfg.api_key.clone(),
                trello_cfg.token.clone(),
                config.board.id.clone(),
                trello_cfg.member_id.clone(),
                config.agent.name.clone(),
            );
            Ok(Box::new(backend))
        }
        other => Err(OrgaError::ConfigError(format!(
            "unsupported backend '{other}'"
        ))),
    }
}
