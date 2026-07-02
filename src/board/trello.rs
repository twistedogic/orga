use async_trait::async_trait;
use std::sync::Arc;

use chrono::DateTime;
use reqwest::Client;
use reqwest::StatusCode;
use serde::Deserialize;

use crate::board::Board;
use crate::error::OrgaError;
use crate::logging::Logger;
use crate::models::{Column, Comment, Member, Ticket, TicketSummary};

pub struct TrelloBackend {
    api_key: String,
    token: String,
    board_id: String,
    member_id: String,
    agent_name: String,
    client: Client,
    logger: Arc<Logger>,
}

impl TrelloBackend {
    pub fn new(
        api_key: String,
        token: String,
        board_id: String,
        member_id: String,
        agent_name: String,
        logger: Arc<Logger>,
    ) -> Result<Self, crate::error::OrgaError> {
        Ok(Self {
            api_key,
            token,
            board_id,
            member_id,
            agent_name,
            client: Client::builder()
                .build()
                .map_err(|e| crate::error::OrgaError::BackendError(e.to_string()))?,
            logger,
        })
    }

    fn auth_params(&self) -> [(&str, &str); 2] {
        [("key", &self.api_key), ("token", &self.token)]
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, OrgaError> {
        let resp = self
            .client
            .get(url)
            .query(&self.auth_params())
            .send()
            .await?;
        let body = self.handle_response(resp).await?;
        serde_json::from_str(&body).map_err(|e| OrgaError::BackendError(e.to_string()))
    }

    async fn post_form(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<serde_json::Value, OrgaError> {
        let mut all: Vec<(&str, &str)> = self.auth_params().to_vec();
        all.extend_from_slice(params);
        let resp = self.client.post(url).query(&all).send().await?;
        let body = self.handle_response(resp).await?;
        serde_json::from_str(&body).map_err(|e| OrgaError::BackendError(e.to_string()))
    }

    async fn handle_response(&self, resp: reqwest::Response) -> Result<String, OrgaError> {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        match status {
            StatusCode::TOO_MANY_REQUESTS => {
                self.logger
                    .error(&format!("Trello HTTP {status}\nBody: {body}"));
                Err(OrgaError::RateLimited)
            }
            StatusCode::UNAUTHORIZED => {
                self.logger
                    .error(&format!("Trello HTTP {status}\nBody: {body}"));
                Err(OrgaError::Unauthorized("invalid Trello credentials".into()))
            }
            StatusCode::NOT_FOUND => {
                self.logger
                    .error(&format!("Trello HTTP {status}\nBody: {body}"));
                Err(OrgaError::NotFound("resource not found".into()))
            }
            s if s.is_client_error() || s.is_server_error() => {
                self.logger
                    .error(&format!("Trello HTTP {status}\nBody: {body}"));
                Err(OrgaError::BackendError(format!(
                    "Trello returned HTTP {status}"
                )))
            }
            _ => Ok(body),
        }
    }

    async fn board_lists(&self) -> Result<Vec<TrelloList>, OrgaError> {
        let url = format!("https://api.trello.com/1/boards/{}/lists", self.board_id);
        self.get(&url).await
    }

    async fn resolve_member_id(&self, username: &str) -> Result<String, OrgaError> {
        let username = username.trim_start_matches('@');
        let url = format!("https://api.trello.com/1/members/{username}");
        let resp: TrelloMember = self.get(&url).await?;
        Ok(resp.id)
    }

    fn card_to_summary(&self, card: &TrelloCard, list_name: String) -> TicketSummary {
        let actions = card.actions.as_deref().unwrap_or_default();

        let creator: Option<Member> = actions
            .iter()
            .find(|a| a.action_type == "createCard")
            .and_then(|a| a.member_creator.as_ref())
            .map(|m| Member {
                id: m.id.clone(),
                username: m.username.clone(),
                full_name: m.full_name.clone(),
            });

        let last_commenter_is_agent = actions
            .iter()
            .filter(|a| a.action_type == "commentCard")
            .max_by_key(|a| a.date.as_str())
            .and_then(|a| a.data.as_ref())
            .and_then(|d| d.text.as_deref())
            .map(|t| parse_agent_tag(t).1.is_some())
            .unwrap_or(false);

        let labels: Vec<String> = card
            .labels
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|l| l.name.clone())
            .filter(|n| !n.is_empty())
            .collect();

        TicketSummary {
            id: card.id.clone(),
            title: card.name.clone(),
            description: card.desc.clone().unwrap_or_default(),
            list_id: card.id_list.clone(),
            list_name,
            url: card.url.clone(),
            completed: card.closed,
            creator,
            last_commenter_is_agent,
            labels,
        }
    }

    fn card_to_ticket(&self, card: TrelloCard, list_name: String) -> Result<Ticket, OrgaError> {
        let actions = card.actions.unwrap_or_default();

        let creator: Option<Member> = actions
            .iter()
            .find(|a| a.action_type == "createCard")
            .and_then(|a| a.member_creator.as_ref())
            .map(|m| Member {
                id: m.id.clone(),
                username: m.username.clone(),
                full_name: m.full_name.clone(),
            });

        let comments: Vec<Comment> = actions
            .into_iter()
            .filter(|a| a.action_type == "commentCard")
            .filter_map(|a| {
                let data = a.data?;
                let text = data.text?;
                let member = a.member_creator?;
                let at = DateTime::parse_from_rfc3339(&a.date)
                    .ok()?
                    .with_timezone(&chrono::Utc);
                let (content, agent_name) = parse_agent_tag(&text);
                Some(Comment {
                    id: a.id,
                    at,
                    who: Member {
                        id: member.id,
                        username: member.username,
                        full_name: member.full_name,
                    },
                    content,
                    agent_name,
                })
            })
            .collect();

        let mut comments: Vec<Comment> = comments;
        comments.sort_by_key(|c| c.at);

        let last_commenter_is_agent = comments
            .last()
            .map(|c| c.agent_name.is_some())
            .unwrap_or(false);

        let assignees: Vec<Member> = card
            .members
            .unwrap_or_default()
            .into_iter()
            .map(|m| Member {
                id: m.id,
                username: m.username,
                full_name: m.full_name,
            })
            .collect();

        let labels: Vec<String> = card
            .labels
            .unwrap_or_default()
            .into_iter()
            .map(|l| l.name)
            .filter(|n| !n.is_empty())
            .collect();

        Ok(Ticket {
            summary: TicketSummary {
                id: card.id,
                title: card.name,
                description: card.desc.unwrap_or_default(),
                list_id: card.id_list,
                list_name,
                url: card.url,
                completed: card.closed,
                creator,
                last_commenter_is_agent,
                labels,
            },
            assignees,
            sub_tickets: vec![],
            comments,
            comment_compaction: None,
            compaction_suggested: false,
        })
    }

    async fn get_list_name(&self, list_id: &str) -> Result<String, OrgaError> {
        let url = format!("https://api.trello.com/1/lists/{list_id}");
        let list: TrelloList = self.get(&url).await?;
        Ok(list.name)
    }

    async fn get_or_create_checklist(
        &self,
        card_id: &str,
        name: &str,
    ) -> Result<String, OrgaError> {
        let url = format!("https://api.trello.com/1/cards/{card_id}/checklists");
        let lists: Vec<TrelloChecklist> = self.get(&url).await?;
        if let Some(existing) = lists.into_iter().find(|cl| cl.name == name) {
            return Ok(existing.id);
        }
        let url = "https://api.trello.com/1/checklists";
        let resp = self
            .post_form(url, &[("idCard", card_id), ("name", name)])
            .await?;
        resp["id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| OrgaError::BackendError("no id in checklist response".into()))
    }
}

#[async_trait]
impl Board for TrelloBackend {
    async fn list_assigned(&self) -> Result<Vec<TicketSummary>, OrgaError> {
        let url = format!("https://api.trello.com/1/members/{}/cards", self.member_id);
        let resp = self
            .client
            .get(&url)
            .query(&self.auth_params())
            .query(&[("filter", "all"), ("actions", "commentCard,createCard")])
            .send()
            .await?;
        let body = self.handle_response(resp).await?;
        let cards: Vec<TrelloCard> =
            serde_json::from_str(&body).map_err(|e| OrgaError::BackendError(e.to_string()))?;

        let cards: Vec<TrelloCard> = cards
            .into_iter()
            .filter(|c| c.id_board == self.board_id)
            .collect();

        let mut summaries = Vec::new();
        for card in cards {
            let list_name = self.get_list_name(&card.id_list).await.unwrap_or_default();
            summaries.push(self.card_to_summary(&card, list_name));
        }
        Ok(summaries)
    }

    async fn get_ticket(&self, id: &str) -> Result<Ticket, OrgaError> {
        let url = format!("https://api.trello.com/1/cards/{id}");
        let resp = self
            .client
            .get(&url)
            .query(&self.auth_params())
            .query(&[
                ("checklists", "all"),
                ("members", "true"),
                ("actions", "commentCard,createCard"),
            ])
            .send()
            .await?;
        let body = self.handle_response(resp).await?;
        let card: TrelloCard =
            serde_json::from_str(&body).map_err(|e| OrgaError::BackendError(e.to_string()))?;
        let list_name = self.get_list_name(&card.id_list).await?;
        self.card_to_ticket(card, list_name)
    }

    async fn comment(&self, id: &str, text: &str) -> Result<(), OrgaError> {
        if text.is_empty() {
            return Err(OrgaError::BackendError(
                "comment text cannot be empty".into(),
            ));
        }
        let tagged = append_agent_tag(text, &self.agent_name);
        let url = format!("https://api.trello.com/1/cards/{id}/actions/comments");
        self.post_form(&url, &[("text", &tagged)]).await?;
        Ok(())
    }

    async fn assign(&self, id: &str, username: &str) -> Result<(), OrgaError> {
        let member_id = self.resolve_member_id(username).await?;
        let url = format!("https://api.trello.com/1/cards/{id}/idMembers");
        self.post_form(&url, &[("value", &member_id)]).await?;
        Ok(())
    }

    async fn create_sub(
        &self,
        parent_id: &str,
        title: &str,
        description: Option<&str>,
        list: Option<&str>,
    ) -> Result<Ticket, OrgaError> {
        let list_id = if let Some(list_name) = list {
            let columns = self.list_columns().await?;
            columns
                .into_iter()
                .find(|c| c.name.eq_ignore_ascii_case(list_name))
                .map(|c| c.id)
                .ok_or_else(|| OrgaError::NotFound(format!("list '{list_name}'")))?
        } else {
            let parent = self.get_ticket(parent_id).await?;
            parent.summary.list_id
        };
        let url = "https://api.trello.com/1/cards";
        let mut params: Vec<(&str, &str)> = vec![("name", title), ("idList", &list_id)];
        let desc_owned: String;
        if let Some(desc) = description {
            desc_owned = desc.to_string();
            params.push(("desc", &desc_owned));
        }
        let resp = self.post_form(url, &params).await?;
        let sub_id = resp["id"]
            .as_str()
            .ok_or_else(|| OrgaError::BackendError("no id in card response".into()))?;
        let sub_url = resp["url"].as_str().unwrap_or(sub_id);

        let checklist_id = self.get_or_create_checklist(parent_id, "Sub-tasks").await?;
        let item_text = format!("{title} - {sub_url}");
        let cl_url = format!("https://api.trello.com/1/checklists/{checklist_id}/checkItems");
        self.post_form(&cl_url, &[("name", &item_text)]).await?;

        self.get_ticket(sub_id).await
    }

    async fn list_columns(&self) -> Result<Vec<Column>, OrgaError> {
        Ok(self
            .board_lists()
            .await?
            .into_iter()
            .map(|l| Column {
                id: l.id,
                name: l.name,
            })
            .collect())
    }

    async fn whoami(&self) -> Result<Member, OrgaError> {
        let url = format!("https://api.trello.com/1/members/{}", self.member_id);
        let resp = self
            .client
            .get(&url)
            .query(&self.auth_params())
            .query(&[("fields", "id,username,fullName")])
            .send()
            .await?;
        let body = self.handle_response(resp).await?;
        let m: TrelloMember =
            serde_json::from_str(&body).map_err(|e| OrgaError::BackendError(e.to_string()))?;
        Ok(Member {
            id: m.id,
            username: m.username,
            full_name: m.full_name,
        })
    }

    async fn return_ticket(&self, id: &str, comment: Option<&str>) -> Result<(), OrgaError> {
        let ticket = self.get_ticket(id).await?;
        let creator = ticket
            .summary
            .creator
            .ok_or_else(|| OrgaError::BackendError("ticket has no known creator".into()))?;
        if let Some(text) = comment {
            self.comment(id, text).await?;
        }
        self.assign(id, &creator.username).await?;
        Ok(())
    }
}

fn append_agent_tag(text: &str, agent_name: &str) -> String {
    if agent_name.is_empty() {
        return text.to_string();
    }
    format!("{text}\n\n_[orga:{agent_name}]_")
}

fn parse_agent_tag(text: &str) -> (String, Option<String>) {
    if let Some(pos) = text.rfind("\n\n_[orga:") {
        let suffix = &text[pos + 2..];
        if suffix.starts_with("_[orga:") && suffix.ends_with("]_") {
            let inner = &suffix[7..suffix.len() - 2];
            if !inner.is_empty() {
                return (text[..pos].to_string(), Some(inner.to_string()));
            }
        }
    }
    (text.to_string(), None)
}

#[derive(Debug, Deserialize)]
struct TrelloList {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrelloCard {
    id: String,
    name: String,
    desc: Option<String>,
    id_list: String,
    id_board: String,
    url: String,
    closed: bool,
    #[allow(dead_code)]
    checklists: Option<Vec<TrelloChecklist>>,
    members: Option<Vec<TrelloMember>>,
    actions: Option<Vec<TrelloAction>>,
    labels: Option<Vec<TrelloLabel>>,
}

#[derive(Debug, Deserialize)]
struct TrelloLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrelloChecklist {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    check_items: Vec<TrelloCheckItem>,
}

#[derive(Debug, Deserialize)]
struct TrelloCheckItem {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    state: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrelloMember {
    id: String,
    username: String,
    full_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrelloAction {
    id: String,
    #[serde(rename = "type")]
    action_type: String,
    date: String,
    data: Option<TrelloActionData>,
    member_creator: Option<TrelloMember>,
}

#[derive(Debug, Deserialize)]
struct TrelloActionData {
    text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_agent_tag_adds_suffix() {
        let result = append_agent_tag("hello", "agent-1");
        assert_eq!(result, "hello\n\n_[orga:agent-1]_");
    }

    #[test]
    fn append_agent_tag_empty_name_unchanged() {
        let result = append_agent_tag("hello", "");
        assert_eq!(result, "hello");
    }

    #[test]
    fn parse_agent_tag_strips_and_extracts() {
        let text = "need more context\n\n_[orga:agent-1]_";
        let (content, agent_name) = parse_agent_tag(text);
        assert_eq!(content, "need more context");
        assert_eq!(agent_name, Some("agent-1".to_string()));
    }

    #[test]
    fn parse_agent_tag_no_tag_unchanged() {
        let text = "just a normal comment";
        let (content, agent_name) = parse_agent_tag(text);
        assert_eq!(content, "just a normal comment");
        assert_eq!(agent_name, None);
    }

    fn make_card(actions: Vec<TrelloAction>) -> TrelloCard {
        TrelloCard {
            id: "card1".into(),
            name: "Test".into(),
            desc: None,
            id_list: "list1".into(),
            id_board: "board1".into(),
            url: "https://example.com".into(),
            closed: false,
            checklists: None,
            members: None,
            actions: Some(actions),
            labels: None,
        }
    }

    fn make_comment_action(date: &str, text: &str) -> TrelloAction {
        TrelloAction {
            id: "a1".into(),
            action_type: "commentCard".into(),
            date: date.into(),
            data: Some(TrelloActionData {
                text: Some(text.into()),
            }),
            member_creator: Some(TrelloMember {
                id: "m1".into(),
                username: "user".into(),
                full_name: "User".into(),
            }),
        }
    }

    fn make_backend() -> TrelloBackend {
        use std::path::Path;
        let logger = Arc::new(Logger::new(Path::new("/dev/null"), false));
        TrelloBackend::new(
            "key".into(),
            "token".into(),
            "board1".into(),
            "member1".into(),
            "agent-1".into(),
            logger,
        )
        .unwrap()
    }

    #[test]
    fn card_to_summary_last_commenter_is_agent_true_when_tagged() {
        let backend = make_backend();
        let card = make_card(vec![
            make_comment_action("2024-01-01T00:00:00Z", "human comment"),
            make_comment_action("2024-01-02T00:00:00Z", "agent reply\n\n_[orga:agent-1]_"),
        ]);
        let summary = backend.card_to_summary(&card, "To Do".into());
        assert!(summary.last_commenter_is_agent);
    }

    #[test]
    fn card_to_summary_last_commenter_is_agent_false_when_not_tagged() {
        let backend = make_backend();
        let card = make_card(vec![
            make_comment_action("2024-01-01T00:00:00Z", "agent reply\n\n_[orga:agent-1]_"),
            make_comment_action("2024-01-02T00:00:00Z", "human reply"),
        ]);
        let summary = backend.card_to_summary(&card, "To Do".into());
        assert!(!summary.last_commenter_is_agent);
    }

    #[test]
    fn card_to_summary_last_commenter_is_agent_false_when_no_comments() {
        let backend = make_backend();
        let card = make_card(vec![]);
        let summary = backend.card_to_summary(&card, "To Do".into());
        assert!(!summary.last_commenter_is_agent);
    }
}
