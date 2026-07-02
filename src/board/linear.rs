use async_trait::async_trait;
use std::sync::Arc;

use chrono::DateTime;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::board::Board;
use crate::board::agent_tag::{append_agent_tag, parse_agent_tag};
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
    pub async fn new(
        api_key: String,
        team_id: String,
        agent_name: String,
        logger: Arc<Logger>,
    ) -> Result<Self, OrgaError> {
        let client = Client::builder()
            .build()
            .map_err(|e| OrgaError::BackendError(e.to_string()))?;
        let backend = Self {
            api_key,
            team_id,
            agent_name,
            viewer: Member {
                id: String::new(),
                username: String::new(),
                full_name: String::new(),
            },
            client,
            logger,
        };
        let viewer = backend.resolve_viewer().await?;
        Ok(Self { viewer, ..backend })
    }

    async fn gql<T: for<'de> Deserialize<'de>>(
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
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::BAD_REQUEST
        {
            let msg = serde_json::from_str::<GqlResponse>(&body)
                .ok()
                .and_then(|r| r.errors)
                .and_then(|e| e.into_iter().next())
                .map(|e| e.message)
                .unwrap_or_else(|| "invalid Linear API key".into());
            self.logger
                .error(&format!("Linear HTTP {status}\nBody: {body}"));
            return Err(OrgaError::Unauthorized(msg));
        }
        if status.is_client_error() || status.is_server_error() {
            self.logger
                .error(&format!("Linear HTTP {status}\nBody: {body}"));
            return Err(OrgaError::BackendError(format!(
                "Linear returned HTTP {status}"
            )));
        }

        let parsed: GqlResponse =
            serde_json::from_str(&body).map_err(|e| OrgaError::BackendError(e.to_string()))?;

        if let Some(first) = parsed.errors.and_then(|e| e.into_iter().next()) {
            self.logger
                .error(&format!("Linear GQL error: {}", first.message));
            return Err(OrgaError::BackendError(first.message));
        }

        let data = parsed
            .data
            .ok_or_else(|| OrgaError::BackendError("Linear returned no data".into()))?;

        serde_json::from_value(data).map_err(|e| OrgaError::BackendError(e.to_string()))
    }

    async fn resolve_viewer(&self) -> Result<Member, OrgaError> {
        #[derive(Deserialize)]
        struct Resp {
            viewer: LinearUser,
        }
        let resp: Resp = self
            .gql("query { viewer { id displayName } }", serde_json::json!({}))
            .await?;
        Ok(Member {
            id: resp.viewer.id.clone(),
            username: resp.viewer.display_name.clone(),
            full_name: resp.viewer.display_name,
        })
    }

    async fn team_states(&self) -> Result<Vec<LinearState>, OrgaError> {
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
        let resp: Resp = self.gql(&query, serde_json::json!({})).await?;
        Ok(resp.team.states.nodes)
    }

    async fn resolve_user_id(&self, username: &str) -> Result<String, OrgaError> {
        let username = username.trim_start_matches('@');
        #[derive(Deserialize)]
        struct Resp {
            users: Nodes<LinearUser>,
        }
        let resp: Resp = self
            .gql(
                "query($name: String!) {
                users(filter: { displayName: { eq: $name } }) {
                    nodes { id displayName }
                }
            }",
                serde_json::json!({ "name": username }),
            )
            .await?;
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
                let completed = is_completed_state(&child.state);
                let list_name = linear_state_name(&child.state);
                let list_id = linear_state_id(&child.state);
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
                let at = DateTime::parse_from_rfc3339(&c.created_at)
                    .ok()?
                    .with_timezone(&chrono::Utc);
                let user = c.user?;
                let (content, agent_name) = parse_agent_tag(&c.body);
                Some(Comment {
                    id: c.id,
                    at,
                    who: member_from_user(&user),
                    content,
                    agent_name,
                })
            })
            .collect();
        comments.sort_by_key(|c| c.at);

        let last_commenter_is_agent = comments
            .last()
            .map(|c| c.agent_name.is_some())
            .unwrap_or(false);

        let creator = issue.creator.as_ref().map(member_from_user);

        let assignees: Vec<Member> = issue
            .assignee
            .map(|u| member_from_user(&u))
            .into_iter()
            .collect();

        let state_name = linear_state_name(&issue.state);
        let state_id = linear_state_id(&issue.state);
        let completed = is_completed_state(&issue.state);

        let labels = linear_label_names(issue.labels.as_ref());

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
        let state_name = linear_state_name(&issue.state);
        let state_id = linear_state_id(&issue.state);
        let completed = is_completed_state(&issue.state);

        let creator = issue.creator.as_ref().map(member_from_user);

        let last_commenter_is_agent = {
            let mut dated: Vec<(DateTime<chrono::Utc>, &LinearCommentSummary)> = issue
                .comments
                .nodes
                .iter()
                .filter_map(|c| {
                    let at = DateTime::parse_from_rfc3339(&c.created_at)
                        .ok()?
                        .with_timezone(&chrono::Utc);
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

        let labels = linear_label_names(issue.labels.as_ref());

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

#[async_trait]
impl Board for LinearBackend {
    async fn whoami(&self) -> Result<Member, OrgaError> {
        Ok(self.viewer.clone())
    }

    async fn list_columns(&self) -> Result<Vec<Column>, OrgaError> {
        Ok(self
            .team_states()
            .await?
            .into_iter()
            .map(|s| Column {
                id: s.id,
                name: s.name,
            })
            .collect())
    }

    async fn list_assigned(&self) -> Result<Vec<TicketSummary>, OrgaError> {
        #[derive(Deserialize)]
        struct Resp {
            issues: Nodes<LinearIssueSummary>,
        }
        let query = format!(
            "{{ issues(filter: {{ team: {{ id: {{ eq: \"{}\" }} }} assignee: {{ id: {{ eq: \"{}\" }} }} }}) {{ nodes {{ id title description url state {{ id name type }} creator {{ id displayName }} comments {{ nodes {{ id body createdAt }} }} labels {{ nodes {{ name }} }} }} }} }}",
            self.team_id, self.viewer.id
        );
        let resp: Resp = self.gql(&query, serde_json::json!({})).await?;
        Ok(resp
            .issues
            .nodes
            .iter()
            .map(|i| self.linear_issue_to_summary(i))
            .collect())
    }

    async fn get_ticket(&self, id: &str) -> Result<Ticket, OrgaError> {
        #[derive(Deserialize)]
        struct Resp {
            issue: LinearIssue,
        }
        let query = format!(
            "{{ issue(id: \"{id}\") {{ id title description url state {{ id name type }} creator {{ id displayName }} assignee {{ id displayName }} comments {{ nodes {{ id body createdAt user {{ id displayName }} }} }} children {{ nodes {{ id title url state {{ id name type }} }} }} labels {{ nodes {{ name }} }} }} }}"
        );
        let resp: Resp = self.gql(&query, serde_json::json!({})).await?;
        Ok(self.linear_issue_to_ticket(resp.issue))
    }

    async fn comment(&self, id: &str, text: &str) -> Result<(), OrgaError> {
        if text.is_empty() {
            return Err(OrgaError::BackendError(
                "comment text cannot be empty".into(),
            ));
        }
        let tagged = append_agent_tag(text, &self.agent_name);
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Resp {
            #[serde(rename = "commentCreate")]
            comment_create: SuccessResp,
        }
        let query = format!(
            "mutation($body: String!) {{ commentCreate(input: {{ issueId: \"{id}\", body: $body }}) {{ success }} }}"
        );
        let _: Resp = self
            .gql(&query, serde_json::json!({ "body": tagged }))
            .await?;
        Ok(())
    }

    async fn assign(&self, id: &str, username: &str) -> Result<(), OrgaError> {
        let user_id = self.resolve_user_id(username).await?;
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Resp {
            #[serde(rename = "issueUpdate")]
            issue_update: SuccessResp,
        }
        let query = format!(
            "mutation {{ issueUpdate(id: \"{id}\", input: {{ assigneeId: \"{user_id}\" }}) {{ success }} }}"
        );
        let _: Resp = self.gql(&query, serde_json::json!({})).await?;
        Ok(())
    }

    async fn create_sub(
        &self,
        parent_id: &str,
        title: &str,
        description: Option<&str>,
        list: Option<&str>,
    ) -> Result<Ticket, OrgaError> {
        let state_id = if let Some(list_name) = list {
            let states = self.team_states().await?;
            states
                .into_iter()
                .find(|s| s.name.eq_ignore_ascii_case(list_name))
                .map(|s| s.id)
                .ok_or_else(|| OrgaError::NotFound(format!("list '{list_name}'")))?
        } else {
            let parent = self.get_ticket(parent_id).await?;
            parent.summary.list_id
        };
        let sub_id = self
            .create_sub_issue(parent_id, title, description, &state_id)
            .await?;
        self.get_ticket(&sub_id).await
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

impl LinearBackend {
    async fn create_sub_issue(
        &self,
        parent_id: &str,
        title: &str,
        description: Option<&str>,
        state_id: &str,
    ) -> Result<String, OrgaError> {
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
        let desc_field = if description.is_some() {
            ", description: $description"
        } else {
            ""
        };
        let query = format!(
            "mutation($title: String!{}) {{ issueCreate(input: {{ teamId: \"{tid}\", parentId: \"{parent_id}\", stateId: \"{state_id}\", title: $title{desc_field} }}) {{ issue {{ id }} }} }}",
            if description.is_some() {
                ", $description: String"
            } else {
                ""
            }
        );
        let mut vars = serde_json::json!({ "title": title });
        if let Some(desc) = description {
            vars["description"] = serde_json::Value::String(desc.to_string());
        }
        let resp: Resp = self.gql(&query, vars).await?;
        Ok(resp.issue_create.issue.id)
    }
}

/// Build a `Member` from a `LinearUser` — single source of truth for the
/// `id, displayName, displayName` triple that both ticket-shape converters
/// built by hand.
fn member_from_user(user: &LinearUser) -> Member {
    Member {
        id: user.id.clone(),
        username: user.display_name.clone(),
        full_name: user.display_name.clone(),
    }
}

fn linear_state_name(state: &Option<LinearState>) -> String {
    state.as_ref().map(|s| s.name.clone()).unwrap_or_default()
}

fn linear_state_id(state: &Option<LinearState>) -> String {
    state.as_ref().map(|s| s.id.clone()).unwrap_or_default()
}

fn is_completed_state(state: &Option<LinearState>) -> bool {
    state
        .as_ref()
        .map(|s| {
            s.state_type.as_deref() == Some("completed")
                || s.state_type.as_deref() == Some("cancelled")
        })
        .unwrap_or(false)
}

fn linear_label_names(labels: Option<&Nodes<LinearLabel>>) -> Vec<String> {
    labels
        .map(|n| {
            n.nodes
                .iter()
                .map(|l| l.name.clone())
                .filter(|n| !n.is_empty())
                .collect()
        })
        .unwrap_or_default()
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

    fn make_summary_comment(body: &str, created_at: &str) -> LinearCommentSummary {
        LinearCommentSummary {
            body: body.into(),
            created_at: created_at.into(),
        }
    }

    fn make_issue_summary(comments: Vec<LinearCommentSummary>) -> LinearIssueSummary {
        LinearIssueSummary {
            id: "issue-1".into(),
            title: "Test".into(),
            description: None,
            url: "https://linear.app/test/issue/TEST-1".into(),
            state: Some(LinearState {
                id: "state-1".into(),
                name: "Todo".into(),
                state_type: Some("unstarted".into()),
            }),
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
            viewer: Member {
                id: "viewer-1".into(),
                username: "agent".into(),
                full_name: "Agent".into(),
            },
            client: Client::builder().build().unwrap(),
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
            state: Some(LinearState {
                id: "state-1".into(),
                name: "Todo".into(),
                state_type: Some("unstarted".into()),
            }),
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
                state: Some(LinearState {
                    id: "s1".into(),
                    name: "Done".into(),
                    state_type: Some("completed".into()),
                }),
            },
            LinearChildIssue {
                id: "sub-2".into(),
                title: "Write test".into(),
                url: "https://linear.app/sub-2".into(),
                state: Some(LinearState {
                    id: "s2".into(),
                    name: "Todo".into(),
                    state_type: Some("unstarted".into()),
                }),
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

    fn make_state(id: &str, name: &str, kind: Option<&str>) -> LinearState {
        LinearState {
            id: id.into(),
            name: name.into(),
            state_type: kind.map(|s| s.to_string()),
        }
    }

    fn make_user(id: &str, display_name: &str) -> LinearUser {
        LinearUser {
            id: id.into(),
            display_name: display_name.into(),
        }
    }

    #[test]
    fn member_from_user_copies_id_and_display_name_to_all_fields() {
        let m = member_from_user(&make_user("u1", "Alice"));
        assert_eq!(m.id, "u1");
        assert_eq!(m.username, "Alice");
        assert_eq!(m.full_name, "Alice");
    }

    #[test]
    fn linear_state_name_and_id_extract_from_present_state() {
        let s = Some(make_state("s1", "Todo", Some("unstarted")));
        assert_eq!(linear_state_name(&s), "Todo");
        assert_eq!(linear_state_id(&s), "s1");
    }

    #[test]
    fn linear_state_name_and_id_return_empty_for_none_state() {
        let s: Option<LinearState> = None;
        assert_eq!(linear_state_name(&s), "");
        assert_eq!(linear_state_id(&s), "");
    }

    #[test]
    fn is_completed_state_true_for_completed_and_cancelled() {
        assert!(is_completed_state(&Some(make_state(
            "s",
            "x",
            Some("completed")
        ))));
        assert!(is_completed_state(&Some(make_state(
            "s",
            "x",
            Some("cancelled")
        ))));
        assert!(!is_completed_state(&Some(make_state(
            "s",
            "x",
            Some("unstarted")
        ))));
        assert!(!is_completed_state(&Some(make_state("s", "x", None))));
        assert!(!is_completed_state(&None));
    }

    #[test]
    fn linear_label_names_filters_empty_and_handles_none() {
        let labels = Nodes {
            nodes: vec![
                LinearLabel { name: "bug".into() },
                LinearLabel { name: "".into() },
                LinearLabel {
                    name: "auth".into(),
                },
            ],
        };
        assert_eq!(
            linear_label_names(Some(&labels)),
            vec!["bug".to_string(), "auth".to_string()]
        );
        assert!(linear_label_names(None).is_empty());
        assert!(linear_label_names(Some(&Nodes { nodes: vec![] })).is_empty());
    }
}
