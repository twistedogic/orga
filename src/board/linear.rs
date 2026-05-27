use std::sync::Arc;

use chrono::DateTime;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::board::Board;
use crate::error::OrgaError;
use crate::logging::Logger;
use crate::models::{Column, Comment, Member, Ticket, TicketSummary};

pub struct LinearBackend {
    api_key: String,
    team_id: String,
    agent_name: String,
    viewer: Member,
    client: Client,
    logger: Arc<Logger>,
}

impl LinearBackend {
    pub fn new(
        api_key: String,
        team_id: String,
        agent_name: String,
        logger: Arc<Logger>,
    ) -> Result<Self, OrgaError> {
        let client = Client::new();
        let backend = Self {
            api_key,
            team_id,
            agent_name,
            viewer: Member { id: String::new(), username: String::new(), full_name: String::new() },
            client,
            logger,
        };
        let viewer = backend.resolve_viewer()?;
        Ok(Self { viewer, ..backend })
    }

    fn gql<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T, OrgaError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            query: &'a str,
            variables: serde_json::Value,
        }
        #[derive(Deserialize)]
        struct GqlError {
            message: String,
        }
        #[derive(Deserialize)]
        struct GqlResponse {
            data: Option<serde_json::Value>,
            errors: Option<Vec<GqlError>>,
        }

        let resp = self
            .client
            .post("https://api.linear.app/graphql")
            .header("Authorization", self.api_key.as_str())
            .header("Content-Type", "application/json")
            .json(&Payload { query, variables })
            .send()?;

        let status = resp.status();
        let body = resp.text().unwrap_or_default();

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::BAD_REQUEST {
            let msg = serde_json::from_str::<GqlResponse>(&body)
                .ok()
                .and_then(|r| r.errors)
                .and_then(|e| e.into_iter().next())
                .map(|e| e.message)
                .unwrap_or_else(|| "invalid Linear API key".into());
            self.logger.error(&format!("Linear HTTP {status}\nBody: {body}"));
            return Err(OrgaError::Unauthorized(msg));
        }
        if status.is_client_error() || status.is_server_error() {
            self.logger.error(&format!("Linear HTTP {status}\nBody: {body}"));
            return Err(OrgaError::BackendError(format!("Linear returned HTTP {status}")));
        }

        let parsed: GqlResponse =
            serde_json::from_str(&body).map_err(|e| OrgaError::BackendError(e.to_string()))?;

        if let Some(first) = parsed.errors.and_then(|e| e.into_iter().next()) {
            self.logger.error(&format!("Linear GQL error: {}", first.message));
            return Err(OrgaError::BackendError(first.message));
        }

        let data = parsed
            .data
            .ok_or_else(|| OrgaError::BackendError("Linear returned no data".into()))?;

        serde_json::from_value(data).map_err(|e| OrgaError::BackendError(e.to_string()))
    }

    fn resolve_viewer(&self) -> Result<Member, OrgaError> {
        #[derive(Deserialize)]
        struct Resp {
            viewer: LinearUser,
        }
        let resp: Resp = self.gql(
            "query { viewer { id displayName } }",
            serde_json::json!({}),
        )?;
        Ok(Member {
            id: resp.viewer.id.clone(),
            username: resp.viewer.display_name.clone(),
            full_name: resp.viewer.display_name,
        })
    }

    fn resolve_state_id(&self, state_name: &str) -> Result<String, OrgaError> {
        let states = self.team_states()?;
        states
            .into_iter()
            .find(|s| s.name.eq_ignore_ascii_case(state_name))
            .map(|s| s.id)
            .ok_or_else(|| OrgaError::NotFound(format!("workflow state '{state_name}'")))
    }

    fn team_states(&self) -> Result<Vec<LinearState>, OrgaError> {
        #[derive(Deserialize)]
        struct Resp {
            team: TeamStatesResp,
        }
        #[derive(Deserialize)]
        struct TeamStatesResp {
            states: Nodes<LinearState>,
        }
        let query = format!(
            "{{ team(id: \"{}\") {{ states {{ nodes {{ id name type }} }} }} }}",
            self.team_id
        );
        let resp: Resp = self.gql(&query, serde_json::json!({}))?;
        Ok(resp.team.states.nodes)
    }

    fn resolve_user_id(&self, username: &str) -> Result<String, OrgaError> {
        let username = username.trim_start_matches('@');
        #[derive(Deserialize)]
        struct Resp {
            users: Nodes<LinearUser>,
        }
        let resp: Resp = self.gql(
            "query($name: String!) {
                users(filter: { displayName: { eq: $name } }) {
                    nodes { id displayName }
                }
            }",
            serde_json::json!({ "name": username }),
        )?;
        match resp.users.nodes.len() {
            0 => Err(OrgaError::NotFound(format!("user '{username}'"))),
            1 => Ok(resp.users.nodes.into_iter().next().unwrap().id),
            _ => Err(OrgaError::BackendError(format!(
                "multiple users match display name '{username}'"
            ))),
        }
    }

    fn linear_issue_to_ticket(&self, issue: LinearIssue) -> Ticket {
        let sub_tickets: Vec<TicketSummary> = issue
            .children
            .nodes
            .into_iter()
            .map(|child| {
                let completed = child
                    .state
                    .as_ref()
                    .map(|s| s.state_type.as_deref() == Some("completed") || s.state_type.as_deref() == Some("cancelled"))
                    .unwrap_or(false);
                let list_name = child.state.as_ref().map(|s| s.name.clone()).unwrap_or_default();
                let list_id = child.state.as_ref().map(|s| s.id.clone()).unwrap_or_default();
                TicketSummary {
                    id: child.id,
                    title: child.title,
                    description: String::new(),
                    list_id,
                    list_name,
                    url: child.url,
                    completed,
                    creator: None,
                    last_commenter_is_agent: false,
                    labels: vec![],
                }
            })
            .collect();

        let mut comments: Vec<Comment> = issue
            .comments
            .nodes
            .into_iter()
            .filter_map(|c| {
                let at = DateTime::parse_from_rfc3339(&c.created_at).ok()?.with_timezone(&chrono::Utc);
                let user = c.user?;
                let (content, agent_name) = parse_agent_tag(&c.body);
                Some(Comment {
                    id: c.id,
                    at,
                    who: Member {
                        id: user.id.clone(),
                        username: user.display_name.clone(),
                        full_name: user.display_name,
                    },
                    content,
                    agent_name,
                })
            })
            .collect();
        comments.sort_by_key(|c| c.at);

        let last_commenter_is_agent = comments.last().map(|c| c.agent_name.is_some()).unwrap_or(false);

        let creator = issue.creator.map(|u| Member {
            id: u.id.clone(),
            username: u.display_name.clone(),
            full_name: u.display_name,
        });

        let assignees: Vec<Member> = issue
            .assignee
            .into_iter()
            .map(|u| Member {
                id: u.id.clone(),
                username: u.display_name.clone(),
                full_name: u.display_name,
            })
            .collect();

        let state_name = issue.state.as_ref().map(|s| s.name.clone()).unwrap_or_default();
        let state_id = issue.state.as_ref().map(|s| s.id.clone()).unwrap_or_default();
        let completed = issue
            .state
            .as_ref()
            .map(|s| s.state_type.as_deref() == Some("completed") || s.state_type.as_deref() == Some("cancelled"))
            .unwrap_or(false);

        let labels: Vec<String> = issue
            .labels
            .unwrap_or_else(|| Nodes { nodes: vec![] })
            .nodes
            .into_iter()
            .map(|l| l.name)
            .filter(|n| !n.is_empty())
            .collect();

        Ticket {
            summary: TicketSummary {
                id: issue.id,
                title: issue.title,
                description: issue.description.unwrap_or_default(),
                list_id: state_id,
                list_name: state_name,
                url: issue.url,
                completed,
                creator,
                last_commenter_is_agent,
                labels,
            },
            assignees,
            sub_tickets,
            comments,
            comment_compaction: None,
            compaction_suggested: false,
        }
    }

    fn linear_issue_to_summary(&self, issue: &LinearIssueSummary) -> TicketSummary {
        let state_name = issue.state.as_ref().map(|s| s.name.clone()).unwrap_or_default();
        let state_id = issue.state.as_ref().map(|s| s.id.clone()).unwrap_or_default();
        let completed = issue
            .state
            .as_ref()
            .map(|s| s.state_type.as_deref() == Some("completed") || s.state_type.as_deref() == Some("cancelled"))
            .unwrap_or(false);

        let creator = issue.creator.as_ref().map(|u| Member {
            id: u.id.clone(),
            username: u.display_name.clone(),
            full_name: u.display_name.clone(),
        });

        let last_commenter_is_agent = {
            let mut dated: Vec<(DateTime<chrono::Utc>, &LinearCommentSummary)> = issue
                .comments
                .nodes
                .iter()
                .filter_map(|c| {
                    let at = DateTime::parse_from_rfc3339(&c.created_at).ok()?.with_timezone(&chrono::Utc);
                    Some((at, c))
                })
                .collect();
            dated.sort_by_key(|(at, _)| *at);
            dated
                .last()
                .and_then(|(_, c)| {
                    let (_, agent_name) = parse_agent_tag(&c.body);
                    agent_name
                })
                .is_some()
        };

        let labels: Vec<String> = issue
            .labels
            .as_ref()
            .map(|n| n.nodes.iter().map(|l| l.name.clone()).filter(|n| !n.is_empty()).collect())
            .unwrap_or_default();

        TicketSummary {
            id: issue.id.clone(),
            title: issue.title.clone(),
            description: issue.description.clone().unwrap_or_default(),
            list_id: state_id,
            list_name: state_name,
            url: issue.url.clone(),
            completed,
            creator,
            last_commenter_is_agent,
            labels,
        }
    }
}

impl Board for LinearBackend {
    fn whoami(&self) -> Result<Member, OrgaError> {
        Ok(self.viewer.clone())
    }

    fn list_columns(&self) -> Result<Vec<Column>, OrgaError> {
        Ok(self
            .team_states()?
            .into_iter()
            .map(|s| Column { id: s.id, name: s.name })
            .collect())
    }

    fn list_assigned(&self) -> Result<Vec<TicketSummary>, OrgaError> {
        #[derive(Deserialize)]
        struct Resp {
            issues: Nodes<LinearIssueSummary>,
        }
        let query = format!(
            "{{ issues(filter: {{ team: {{ id: {{ eq: \"{}\" }} }} assignee: {{ id: {{ eq: \"{}\" }} }} }}) {{ nodes {{ id title description url state {{ id name type }} creator {{ id displayName }} comments {{ nodes {{ id body createdAt }} }} labels {{ nodes {{ name }} }} }} }} }}",
            self.team_id, self.viewer.id
        );
        let resp: Resp = self.gql(&query, serde_json::json!({}))?;
        Ok(resp.issues.nodes.iter().map(|i| self.linear_issue_to_summary(i)).collect())
    }

    fn get_ticket(&self, id: &str) -> Result<Ticket, OrgaError> {
        #[derive(Deserialize)]
        struct Resp {
            issue: LinearIssue,
        }
        let query = format!(
            "{{ issue(id: \"{id}\") {{ id title description url state {{ id name type }} creator {{ id displayName }} assignee {{ id displayName }} comments {{ nodes {{ id body createdAt user {{ id displayName }} }} }} children {{ nodes {{ id title url state {{ id name type }} }} }} labels {{ nodes {{ name }} }} }} }}"
        );
        let resp: Resp = self.gql(&query, serde_json::json!({}))?;
        Ok(self.linear_issue_to_ticket(resp.issue))
    }

    fn comment(&self, id: &str, text: &str) -> Result<(), OrgaError> {
        if text.is_empty() {
            return Err(OrgaError::BackendError("comment text cannot be empty".into()));
        }
        let tagged = append_agent_tag(text, &self.agent_name);
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Resp {
            #[serde(rename = "commentCreate")]
            comment_create: SuccessResp,
        }
        let query = format!("mutation($body: String!) {{ commentCreate(input: {{ issueId: \"{id}\", body: $body }}) {{ success }} }}");
        let _: Resp = self.gql(&query, serde_json::json!({ "body": tagged }))?;
        Ok(())
    }

    fn move_ticket(&self, id: &str, list: &str) -> Result<(), OrgaError> {
        let state_id = self.resolve_state_id(list)?;
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Resp {
            #[serde(rename = "issueUpdate")]
            issue_update: SuccessResp,
        }
        let query = format!("mutation {{ issueUpdate(id: \"{id}\", input: {{ stateId: \"{state_id}\" }}) {{ success }} }}");
        let _: Resp = self.gql(&query, serde_json::json!({}))?;
        Ok(())
    }

    fn assign(&self, id: &str, username: &str) -> Result<(), OrgaError> {
        let user_id = self.resolve_user_id(username)?;
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Resp {
            #[serde(rename = "issueUpdate")]
            issue_update: SuccessResp,
        }
        let query = format!("mutation {{ issueUpdate(id: \"{id}\", input: {{ assigneeId: \"{user_id}\" }}) {{ success }} }}");
        let _: Resp = self.gql(&query, serde_json::json!({}))?;
        Ok(())
    }

    fn create_sub(&self, parent_id: &str, title: &str, description: Option<&str>, list: Option<&str>) -> Result<Ticket, OrgaError> {
        let state_id = if let Some(list_name) = list {
            let states = self.team_states()?;
            states
                .into_iter()
                .find(|s| s.name.eq_ignore_ascii_case(list_name))
                .map(|s| s.id)
                .ok_or_else(|| OrgaError::NotFound(format!("list '{list_name}'")))?  
        } else {
            let parent = self.get_ticket(parent_id)?;
            parent.summary.list_id
        };
        let sub_id = self.create_sub_issue(parent_id, title, description, &state_id)?;
        self.get_ticket(&sub_id)
    }

    fn return_ticket(&self, id: &str, comment: Option<&str>) -> Result<(), OrgaError> {
        let ticket = self.get_ticket(id)?;
        let creator = ticket.summary.creator.ok_or_else(|| {
            OrgaError::BackendError("ticket has no known creator".into())
        })?;
        if let Some(text) = comment {
            self.comment(id, text)?;
        }
        self.assign(id, &creator.username)?;
        Ok(())
    }
}

impl LinearBackend {
    fn create_sub_issue(&self, parent_id: &str, title: &str, description: Option<&str>, state_id: &str) -> Result<String, OrgaError> {
        #[derive(Deserialize)]
        struct Resp {
            #[serde(rename = "issueCreate")]
            issue_create: IssueCreateResp,
        }
        #[derive(Deserialize)]
        struct IssueCreateResp {
            issue: IssueId,
        }
        #[derive(Deserialize)]
        struct IssueId {
            id: String,
        }
        let tid = &self.team_id;
        let desc_field = if description.is_some() { ", description: $description" } else { "" };
        let query = format!("mutation($title: String!{}) {{ issueCreate(input: {{ teamId: \"{tid}\", parentId: \"{parent_id}\", stateId: \"{state_id}\", title: $title{desc_field} }}) {{ issue {{ id }} }} }}",
            if description.is_some() { ", $description: String" } else { "" });
        let mut vars = serde_json::json!({ "title": title });
        if let Some(desc) = description {
            vars["description"] = serde_json::Value::String(desc.to_string());
        }
        let resp: Resp = self.gql(&query, vars)?;
        Ok(resp.issue_create.issue.id)
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
struct Nodes<T> {
    nodes: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct LinearUser {
    id: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct LinearState {
    id: String,
    name: String,
    #[serde(rename = "type")]
    state_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LinearComment {
    id: String,
    body: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    user: Option<LinearUser>,
}

#[derive(Debug, Deserialize)]
struct LinearCommentSummary {
    body: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct LinearChildIssue {
    id: String,
    title: String,
    url: String,
    state: Option<LinearState>,
}

#[derive(Debug, Deserialize)]
struct LinearLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct LinearIssue {
    id: String,
    title: String,
    description: Option<String>,
    url: String,
    state: Option<LinearState>,
    creator: Option<LinearUser>,
    assignee: Option<LinearUser>,
    comments: Nodes<LinearComment>,
    children: Nodes<LinearChildIssue>,
    labels: Option<Nodes<LinearLabel>>,
}

#[derive(Debug, Deserialize)]
struct LinearIssueSummary {
    id: String,
    title: String,
    description: Option<String>,
    url: String,
    state: Option<LinearState>,
    creator: Option<LinearUser>,
    comments: Nodes<LinearCommentSummary>,
    labels: Option<Nodes<LinearLabel>>,
}

#[derive(Debug, Deserialize)]
struct SuccessResp {
    #[allow(dead_code)]
    success: bool,
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

    fn make_summary_comment(body: &str, created_at: &str) -> LinearCommentSummary {
        LinearCommentSummary { body: body.into(), created_at: created_at.into() }
    }

    fn make_issue_summary(comments: Vec<LinearCommentSummary>) -> LinearIssueSummary {
        LinearIssueSummary {
            id: "issue-1".into(),
            title: "Test".into(),
            description: None,
            url: "https://linear.app/test/issue/TEST-1".into(),
            state: Some(LinearState { id: "state-1".into(), name: "Todo".into(), state_type: Some("unstarted".into()) }),
            creator: None,
            comments: Nodes { nodes: comments },
            labels: None,
        }
    }

    fn make_backend() -> LinearBackend {
        use std::path::Path;
        let logger = Arc::new(Logger::new(Path::new("/dev/null"), false));
        LinearBackend {
            api_key: "key".into(),
            team_id: "team-1".into(),
            agent_name: "agent-1".into(),
            viewer: Member { id: "viewer-1".into(), username: "agent".into(), full_name: "Agent".into() },
            client: Client::new(),
            logger,
        }
    }

    #[test]
    fn last_commenter_is_agent_true_when_tagged() {
        let backend = make_backend();
        let issue = make_issue_summary(vec![
            make_summary_comment("human comment", "2024-01-01T10:00:00Z"),
            make_summary_comment("agent reply\n\n_[orga:agent-1]_", "2024-01-01T11:00:00Z"),
        ]);
        let summary = backend.linear_issue_to_summary(&issue);
        assert!(summary.last_commenter_is_agent);
    }

    #[test]
    fn last_commenter_is_agent_false_when_not_tagged() {
        let backend = make_backend();
        let issue = make_issue_summary(vec![
            make_summary_comment("agent reply\n\n_[orga:agent-1]_", "2024-01-01T10:00:00Z"),
            make_summary_comment("human reply", "2024-01-01T11:00:00Z"),
        ]);
        let summary = backend.linear_issue_to_summary(&issue);
        assert!(!summary.last_commenter_is_agent);
    }

    #[test]
    fn last_commenter_is_agent_true_when_agent_is_latest_by_timestamp() {
        let backend = make_backend();
        let issue = make_issue_summary(vec![
            make_summary_comment("agent reply\n\n_[orga:agent-1]_", "2024-01-01T12:00:00Z"),
            make_summary_comment("human reply", "2024-01-01T10:00:00Z"),
        ]);
        let summary = backend.linear_issue_to_summary(&issue);
        assert!(summary.last_commenter_is_agent);
    }

    #[test]
    fn last_commenter_is_agent_false_when_no_comments() {
        let backend = make_backend();
        let issue = make_issue_summary(vec![]);
        let summary = backend.linear_issue_to_summary(&issue);
        assert!(!summary.last_commenter_is_agent);
    }

    fn make_full_issue(children: Vec<LinearChildIssue>) -> LinearIssue {
        LinearIssue {
            id: "issue-1".into(),
            title: "Parent".into(),
            description: None,
            url: "https://linear.app/test/issue/TEST-1".into(),
            state: Some(LinearState { id: "state-1".into(), name: "Todo".into(), state_type: Some("unstarted".into()) }),
            creator: None,
            assignee: None,
            comments: Nodes { nodes: vec![] },
            children: Nodes { nodes: children },
            labels: None,
        }
    }

    #[test]
    fn get_ticket_maps_sub_issues_to_sub_tickets() {
        let backend = make_backend();
        let issue = make_full_issue(vec![
            LinearChildIssue {
                id: "sub-1".into(),
                title: "Fix bug".into(),
                url: "https://linear.app/sub-1".into(),
                state: Some(LinearState { id: "s1".into(), name: "Done".into(), state_type: Some("completed".into()) }),
            },
            LinearChildIssue {
                id: "sub-2".into(),
                title: "Write test".into(),
                url: "https://linear.app/sub-2".into(),
                state: Some(LinearState { id: "s2".into(), name: "Todo".into(), state_type: Some("unstarted".into()) }),
            },
        ]);
        let ticket = backend.linear_issue_to_ticket(issue);
        assert_eq!(ticket.sub_tickets.len(), 2);
        assert_eq!(ticket.sub_tickets[0].title, "Fix bug");
        assert!(ticket.sub_tickets[0].completed);
        assert_eq!(ticket.sub_tickets[0].url, "https://linear.app/sub-1");
        assert_eq!(ticket.sub_tickets[1].title, "Write test");
        assert!(!ticket.sub_tickets[1].completed);
    }

    #[test]
    fn get_ticket_no_sub_issues_empty_sub_tickets() {
        let backend = make_backend();
        let issue = make_full_issue(vec![]);
        let ticket = backend.linear_issue_to_ticket(issue);
        assert!(ticket.sub_tickets.is_empty());
    }
}
