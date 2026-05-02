use chrono::DateTime;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;

use crate::board::Board;
use crate::error::OrgaError;
use crate::models::{Checklist, ChecklistItem, Column, Comment, Member, Ticket};

pub struct TrelloBackend {
    api_key: String,
    token: String,
    board_id: String,
    member_id: String,
    client: Client,
}

impl TrelloBackend {
    pub fn new(api_key: String, token: String, board_id: String, member_id: String) -> Self {
        Self {
            api_key,
            token,
            board_id,
            member_id,
            client: Client::new(),
        }
    }

    fn auth_params(&self) -> [(&str, &str); 2] {
        [("key", &self.api_key), ("token", &self.token)]
    }

    fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, OrgaError> {
        let resp = self
            .client
            .get(url)
            .query(&self.auth_params())
            .send()?;
        self.check_status(&resp)?;
        Ok(resp.json().map_err(|e| OrgaError::BackendError(e.to_string()))?)
    }

    fn post_form(&self, url: &str, params: &[(&str, &str)]) -> Result<serde_json::Value, OrgaError> {
        let mut all: Vec<(&str, &str)> = self.auth_params().to_vec();
        all.extend_from_slice(params);
        let resp = self.client.post(url).query(&all).send()?;
        self.check_status(&resp)?;
        Ok(resp.json().map_err(|e| OrgaError::BackendError(e.to_string()))?)
    }

    fn put_form(&self, url: &str, params: &[(&str, &str)]) -> Result<serde_json::Value, OrgaError> {
        let mut all: Vec<(&str, &str)> = self.auth_params().to_vec();
        all.extend_from_slice(params);
        let resp = self.client.put(url).query(&all).send()?;
        self.check_status(&resp)?;
        Ok(resp.json().map_err(|e| OrgaError::BackendError(e.to_string()))?)
    }

    fn check_status(&self, resp: &reqwest::blocking::Response) -> Result<(), OrgaError> {
        match resp.status() {
            StatusCode::TOO_MANY_REQUESTS => Err(OrgaError::RateLimited),
            StatusCode::UNAUTHORIZED => Err(OrgaError::Unauthorized("invalid Trello credentials".into())),
            StatusCode::NOT_FOUND => Err(OrgaError::NotFound("resource not found".into())),
            s if s.is_client_error() || s.is_server_error() => {
                Err(OrgaError::BackendError(format!("Trello returned HTTP {s}")))
            }
            _ => Ok(()),
        }
    }

    fn board_lists(&self) -> Result<Vec<TrelloList>, OrgaError> {
        let url = format!("https://api.trello.com/1/boards/{}/lists", self.board_id);
        self.get(&url)
    }

    fn resolve_list_id(&self, list_name: &str) -> Result<String, OrgaError> {
        let lists = self.board_lists()?;
        lists
            .into_iter()
            .find(|l| l.name.eq_ignore_ascii_case(list_name))
            .map(|l| l.id)
            .ok_or_else(|| OrgaError::NotFound(format!("list '{list_name}'")))
    }

    fn resolve_member_id(&self, username: &str) -> Result<String, OrgaError> {
        let username = username.trim_start_matches('@');
        let url = format!("https://api.trello.com/1/members/{username}");
        let resp: TrelloMember = self.get(&url)?;
        Ok(resp.id)
    }

    fn card_to_ticket(&self, card: TrelloCard, list_name: String) -> Result<Ticket, OrgaError> {
        let checklists: Vec<Checklist> = card
            .checklists
            .unwrap_or_default()
            .into_iter()
            .map(|cl| Checklist {
                id: cl.id,
                name: cl.name,
                items: cl
                    .check_items
                    .into_iter()
                    .map(|i| ChecklistItem {
                        id: i.id,
                        text: i.name,
                        complete: i.state == "complete",
                    })
                    .collect(),
            })
            .collect();

        let comments: Vec<Comment> = card
            .actions
            .unwrap_or_default()
            .into_iter()
            .filter(|a| a.action_type == "commentCard")
            .filter_map(|a| {
                let data = a.data?;
                let text = data.text?;
                let member = a.member_creator?;
                let at = DateTime::parse_from_rfc3339(&a.date)
                    .ok()?
                    .with_timezone(&chrono::Utc);
                Some(Comment {
                    id: a.id,
                    at,
                    who: Member {
                        id: member.id,
                        username: member.username,
                        full_name: member.full_name,
                    },
                    content: text,
                })
            })
            .collect();

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

        Ok(Ticket {
            id: card.id,
            title: card.name,
            description: card.desc.unwrap_or_default(),
            list_id: card.id_list,
            list_name,
            url: card.url,
            completed: card.closed,
            assignees,
            checklists,
            comments,
        })
    }

    fn get_list_name(&self, list_id: &str) -> Result<String, OrgaError> {
        let url = format!("https://api.trello.com/1/lists/{list_id}");
        let list: TrelloList = self.get(&url)?;
        Ok(list.name)
    }

    fn get_or_create_checklist(&self, card_id: &str, name: &str) -> Result<String, OrgaError> {
        let url = format!("https://api.trello.com/1/cards/{card_id}/checklists");
        let lists: Vec<TrelloChecklist> = self.get(&url)?;
        if let Some(existing) = lists.into_iter().find(|cl| cl.name == name) {
            return Ok(existing.id);
        }
        let url = "https://api.trello.com/1/checklists";
        let resp = self.post_form(url, &[("idCard", card_id), ("name", name)])?;
        resp["id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| OrgaError::BackendError("no id in checklist response".into()))
    }
}

impl Board for TrelloBackend {
    fn list_assigned(&self) -> Result<Vec<Ticket>, OrgaError> {
        let url = format!(
            "https://api.trello.com/1/members/{}/cards",
            self.member_id
        );
        let resp = self
            .client
            .get(&url)
            .query(&self.auth_params())
            .query(&[("filter", "all")])
            .send()?;
        self.check_status(&resp)?;
        let cards: Vec<TrelloCard> = resp.json().map_err(|e| OrgaError::BackendError(e.to_string()))?;

        let cards: Vec<TrelloCard> = cards
            .into_iter()
            .filter(|c| c.id_board == self.board_id)
            .collect();

        cards
            .into_iter()
            .map(|card| {
                let list_name = self.get_list_name(&card.id_list).unwrap_or_default();
                self.card_to_ticket(card, list_name)
            })
            .collect()
    }

    fn get_ticket(&self, id: &str) -> Result<Ticket, OrgaError> {
        let url = format!("https://api.trello.com/1/cards/{id}");
        let resp = self
            .client
            .get(&url)
            .query(&self.auth_params())
            .query(&[
                ("checklists", "all"),
                ("members", "true"),
                ("actions", "commentCard"),
            ])
            .send()?;
        self.check_status(&resp)?;
        let card: TrelloCard = resp.json().map_err(|e| OrgaError::BackendError(e.to_string()))?;
        let list_name = self.get_list_name(&card.id_list)?;
        self.card_to_ticket(card, list_name)
    }

    fn comment(&self, id: &str, text: &str) -> Result<(), OrgaError> {
        if text.is_empty() {
            return Err(OrgaError::BackendError("comment text cannot be empty".into()));
        }
        let url = format!("https://api.trello.com/1/cards/{id}/actions/comments");
        self.post_form(&url, &[("text", text)])?;
        Ok(())
    }

    fn assign(&self, id: &str, username: &str) -> Result<(), OrgaError> {
        let member_id = self.resolve_member_id(username)?;
        let url = format!("https://api.trello.com/1/cards/{id}/idMembers");
        self.post_form(&url, &[("value", &member_id)])?;
        Ok(())
    }

    fn move_ticket(&self, id: &str, list: &str) -> Result<(), OrgaError> {
        let list_id = self.resolve_list_id(list)?;
        let url = format!("https://api.trello.com/1/cards/{id}");
        self.put_form(&url, &[("idList", &list_id)])?;
        Ok(())
    }

    fn create_sub(&self, parent_id: &str, title: &str) -> Result<Ticket, OrgaError> {
        let parent = self.get_ticket(parent_id)?;
        let url = "https://api.trello.com/1/cards";
        let resp = self.post_form(
            url,
            &[("name", title), ("idList", &parent.list_id)],
        )?;
        let sub_id = resp["id"]
            .as_str()
            .ok_or_else(|| OrgaError::BackendError("no id in card response".into()))?;
        let sub_url = resp["url"]
            .as_str()
            .unwrap_or(sub_id);

        let checklist_id = self.get_or_create_checklist(parent_id, "Sub-tasks")?;
        let item_text = format!("{title} - {sub_url}");
        let cl_url = format!("https://api.trello.com/1/checklists/{checklist_id}/checkItems");
        self.post_form(&cl_url, &[("name", &item_text)])?;

        self.get_ticket(sub_id)
    }

    fn add_checklist_item(&self, id: &str, text: &str) -> Result<String, OrgaError> {
        let checklist_id = self.get_or_create_checklist(id, "Tasks")?;
        let url = format!("https://api.trello.com/1/checklists/{checklist_id}/checkItems");
        let resp = self.post_form(&url, &[("name", text)])?;
        resp["id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| OrgaError::BackendError("no id in checkItem response".into()))
    }

    fn list_columns(&self) -> Result<Vec<Column>, OrgaError> {
        Ok(self
            .board_lists()?
            .into_iter()
            .map(|l| Column { id: l.id, name: l.name })
            .collect())
    }

    fn check_item(&self, id: &str, item_id: &str) -> Result<(), OrgaError> {
        let url = format!("https://api.trello.com/1/cards/{id}/checklists");
        let lists: Vec<TrelloChecklist> = self.get(&url)?;
        let checklist_id = lists
            .iter()
            .find(|cl| cl.check_items.iter().any(|i| i.id == item_id))
            .map(|cl| cl.id.clone())
            .ok_or_else(|| OrgaError::NotFound(format!("checklist item '{item_id}'")))?;

        let url = format!(
            "https://api.trello.com/1/cards/{id}/checklist/{checklist_id}/checkItem/{item_id}"
        );
        self.put_form(&url, &[("state", "complete")])?;
        Ok(())
    }
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
    checklists: Option<Vec<TrelloChecklist>>,
    members: Option<Vec<TrelloMember>>,
    actions: Option<Vec<TrelloAction>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrelloChecklist {
    id: String,
    name: String,
    check_items: Vec<TrelloCheckItem>,
}

#[derive(Debug, Deserialize)]
struct TrelloCheckItem {
    id: String,
    name: String,
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
