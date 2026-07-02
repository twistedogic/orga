use async_trait::async_trait;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::error::OrgaError;
use crate::logging::Logger;
use crate::models::{Column, Member, Ticket, TicketSummary};

pub mod linear;
pub mod trello;

#[async_trait]
pub trait Board {
    async fn list_assigned(&self) -> Result<Vec<TicketSummary>, OrgaError>;
    async fn get_ticket(&self, id: &str) -> Result<Ticket, OrgaError>;
    async fn comment(&self, id: &str, text: &str) -> Result<(), OrgaError>;
    async fn assign(&self, id: &str, username: &str) -> Result<(), OrgaError>;
    async fn create_sub(
        &self,
        parent_id: &str,
        title: &str,
        description: Option<&str>,
        list: Option<&str>,
    ) -> Result<Ticket, OrgaError>;
    async fn list_columns(&self) -> Result<Vec<Column>, OrgaError>;
    async fn whoami(&self) -> Result<Member, OrgaError>;
    async fn return_ticket(&self, id: &str, comment: Option<&str>) -> Result<(), OrgaError>;
}

pub async fn build_board(
    config: &AppConfig,
    logger: Arc<Logger>,
) -> Result<Box<dyn Board>, OrgaError> {
    match config.board.backend.as_str() {
        "trello" => {
            let trello_cfg = config
                .trello
                .as_ref()
                .ok_or_else(|| OrgaError::ConfigError("[trello] section missing".into()))?;
            let backend = trello::TrelloBackend::new(
                trello_cfg.api_key.clone(),
                trello_cfg.token.clone(),
                trello_cfg.board_id.clone(),
                trello_cfg.member_id.clone(),
                config.agent.name.clone(),
                logger,
            )?;
            Ok(Box::new(backend))
        }
        "linear" => {
            let linear_cfg = config
                .linear
                .as_ref()
                .ok_or_else(|| OrgaError::ConfigError("[linear] section missing".into()))?;
            let backend = linear::LinearBackend::new(
                linear_cfg.api_key.clone(),
                linear_cfg.team_id.clone(),
                config.agent.name.clone(),
                logger,
            )
            .await?;
            Ok(Box::new(backend))
        }
        other => Err(OrgaError::ConfigError(format!(
            "unsupported backend '{other}'"
        ))),
    }
}
