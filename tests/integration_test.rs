use async_trait::async_trait;
use chrono::Utc;

use orga::board::Board;
use orga::error::OrgaError;
use orga::models::{Column, Comment, Member, Ticket, TicketSummary};

struct MockBoard {
    tickets: Vec<Ticket>,
    whoami_member: Member,
}

impl MockBoard {
    fn with_tickets(tickets: Vec<Ticket>) -> Self {
        Self {
            tickets,
            whoami_member: Member {
                id: "agent-id".into(),
                username: "agent-1".into(),
                full_name: "Agent One".into(),
            },
        }
    }
}

#[async_trait]
impl Board for MockBoard {
    async fn list_assigned(&self) -> Result<Vec<TicketSummary>, OrgaError> {
        Ok(self.tickets.iter().map(|t| t.summary.clone()).collect())
    }

    async fn get_ticket(&self, id: &str) -> Result<Ticket, OrgaError> {
        self.tickets
            .iter()
            .find(|t| t.summary.id == id)
            .cloned()
            .ok_or_else(|| OrgaError::NotFound(id.to_string()))
    }

    async fn comment(&self, _id: &str, text: &str) -> Result<(), OrgaError> {
        if text.is_empty() {
            return Err(OrgaError::BackendError("empty comment".into()));
        }
        Ok(())
    }

    async fn assign(&self, _id: &str, _username: &str) -> Result<(), OrgaError> {
        Ok(())
    }

    async fn create_sub(
        &self,
        parent_id: &str,
        title: &str,
        _description: Option<&str>,
        _list: Option<&str>,
    ) -> Result<Ticket, OrgaError> {
        let _parent = self.get_ticket(parent_id).await?;
        Ok(Ticket {
            summary: TicketSummary {
                id: "sub-1".into(),
                title: title.into(),
                description: String::new(),
                list_id: "list-1".into(),
                list_name: "To Do".into(),
                url: "https://trello.com/c/sub-1".into(),
                completed: false,
                creator: None,
                last_commenter_is_agent: false,
                labels: vec![],
            },
            assignees: vec![],
            sub_tickets: vec![],
            comments: vec![],
            comment_compaction: None,
            compaction_suggested: false,
        })
    }

    async fn list_columns(&self) -> Result<Vec<Column>, OrgaError> {
        Ok(vec![
            Column {
                id: "list-1".into(),
                name: "To Do".into(),
            },
            Column {
                id: "list-2".into(),
                name: "In Progress".into(),
            },
        ])
    }

    async fn whoami(&self) -> Result<Member, OrgaError> {
        Ok(self.whoami_member.clone())
    }

    async fn return_ticket(&self, id: &str, _comment: Option<&str>) -> Result<(), OrgaError> {
        let ticket = self.get_ticket(id).await?;
        ticket
            .summary
            .creator
            .ok_or_else(|| OrgaError::BackendError("ticket has no known creator".into()))?;
        Ok(())
    }
}

fn sample_member() -> Member {
    Member {
        id: "u1".into(),
        username: "alice".into(),
        full_name: "Alice".into(),
    }
}

fn sample_ticket() -> Ticket {
    Ticket {
        summary: TicketSummary {
            id: "abc123".into(),
            title: "Fix login bug".into(),
            description: "The login page crashes on submit.".into(),
            list_id: "list-1".into(),
            list_name: "In Progress".into(),
            url: "https://trello.com/c/abc123".into(),
            completed: false,
            last_commenter_is_agent: false,
            creator: Some(Member {
                id: "u2".into(),
                username: "bob".into(),
                full_name: "Bob".into(),
            }),
            labels: vec![],
        },
        assignees: vec![Member {
            id: "m1".into(),
            username: "agent-1".into(),
            full_name: "Agent One".into(),
        }],
        sub_tickets: vec![],
        comments: vec![Comment {
            id: "c1".into(),
            at: Utc::now(),
            who: sample_member(),
            content: "Please fix this ASAP.".into(),
            agent_name: None,
        }],
        comment_compaction: None,
        compaction_suggested: false,
    }
}

fn ticket_no_creator() -> Ticket {
    Ticket {
        summary: TicketSummary {
            id: "no-creator".into(),
            title: "Ticket without creator".into(),
            description: String::new(),
            list_id: "list-1".into(),
            list_name: "To Do".into(),
            url: "https://trello.com/c/no-creator".into(),
            completed: false,
            creator: None,
            last_commenter_is_agent: false,
            labels: vec![],
        },
        assignees: vec![],
        sub_tickets: vec![],
        comments: vec![],
        comment_compaction: None,
        compaction_suggested: false,
    }
}

#[tokio::test]
async fn list_assigned_returns_tickets() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let summaries = board.list_assigned().await.unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, "abc123");
}

#[tokio::test]
async fn list_assigned_empty() {
    let board = MockBoard::with_tickets(vec![]);
    let summaries = board.list_assigned().await.unwrap();
    assert!(summaries.is_empty());
}

#[tokio::test]
async fn get_ticket_found() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let t = board.get_ticket("abc123").await.unwrap();
    assert_eq!(t.summary.title, "Fix login bug");
    assert_eq!(t.comments.len(), 1);
}

#[tokio::test]
async fn get_ticket_not_found() {
    let board = MockBoard::with_tickets(vec![]);
    let err = board.get_ticket("nonexistent").await.unwrap_err();
    assert!(matches!(err, OrgaError::NotFound(_)));
}

#[tokio::test]
async fn comment_empty_fails() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let err = board.comment("abc123", "").await.unwrap_err();
    assert!(matches!(err, OrgaError::BackendError(_)));
}

#[tokio::test]
async fn create_sub_links_to_parent() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let sub = board
        .create_sub("abc123", "Sub-task one", None, None)
        .await
        .unwrap();
    assert_eq!(sub.summary.title, "Sub-task one");
    assert_eq!(sub.summary.list_id, "list-1");
}

#[test]
fn ticket_json_serializable() {
    let t = sample_ticket();
    let json = serde_json::to_string(&t).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["id"], "abc123");
    assert_eq!(parsed["title"], "Fix login bug");
    assert_eq!(parsed["completed"], false);
    assert!(parsed["comments"].is_array());
    assert!(parsed["sub_tickets"].is_array());
}

#[test]
fn ticket_summary_json_has_no_detail_fields() {
    let t = sample_ticket();
    let json = serde_json::to_string(&t.summary).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["id"], "abc123");
    assert_eq!(parsed["title"], "Fix login bug");
    assert!(parsed.get("comments").is_none());
    assert!(parsed.get("sub_tickets").is_none());
    assert!(parsed.get("assignees").is_none());
}

fn completed_ticket() -> Ticket {
    Ticket {
        summary: TicketSummary {
            id: "done1".into(),
            title: "Closed ticket".into(),
            description: String::new(),
            list_id: "list-3".into(),
            list_name: "Done".into(),
            url: "https://trello.com/c/done1".into(),
            completed: true,
            creator: None,
            last_commenter_is_agent: false,
            labels: vec![],
        },
        assignees: vec![],
        sub_tickets: vec![],
        comments: vec![],
        comment_compaction: None,
        compaction_suggested: false,
    }
}

#[tokio::test]
async fn list_assigned_returns_all_tickets() {
    let board = MockBoard::with_tickets(vec![sample_ticket(), completed_ticket()]);
    let summaries = board.list_assigned().await.unwrap();
    assert_eq!(summaries.len(), 2);
}

#[tokio::test]
async fn filter_open_tickets() {
    let board = MockBoard::with_tickets(vec![sample_ticket(), completed_ticket()]);
    let all = board.list_assigned().await.unwrap();
    let open: Vec<_> = all.iter().filter(|t| !t.completed).collect();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].id, "abc123");
}

#[tokio::test]
async fn filter_completed_tickets() {
    let board = MockBoard::with_tickets(vec![sample_ticket(), completed_ticket()]);
    let all = board.list_assigned().await.unwrap();
    let done: Vec<_> = all.iter().filter(|t| t.completed).collect();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].id, "done1");
}

#[tokio::test]
async fn filter_all_tickets() {
    let board = MockBoard::with_tickets(vec![sample_ticket(), completed_ticket()]);
    let all = board.list_assigned().await.unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn completed_ticket_json_has_completed_true() {
    let t = completed_ticket();
    let json = serde_json::to_string(&t.summary).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["completed"], true);
}

#[tokio::test]
async fn whoami_returns_member() {
    let board = MockBoard::with_tickets(vec![]);
    let m = board.whoami().await.unwrap();
    assert_eq!(m.id, "agent-id");
    assert_eq!(m.username, "agent-1");
    assert_eq!(m.full_name, "Agent One");
}

#[tokio::test]
async fn whoami_json_has_expected_fields() {
    let board = MockBoard::with_tickets(vec![]);
    let m = board.whoami().await.unwrap();
    let json = serde_json::to_string(&m).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("id").is_some());
    assert!(parsed.get("username").is_some());
    assert!(parsed.get("full_name").is_some());
}

#[test]
fn ticket_show_json_has_creator_field() {
    let t = sample_ticket();
    let json = serde_json::to_string(&t).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("creator").is_some());
    assert_eq!(parsed["creator"]["username"], "bob");
}

#[test]
fn ticket_show_json_creator_null_when_absent() {
    let t = ticket_no_creator();
    let json = serde_json::to_string(&t).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["creator"].is_null());
}

#[tokio::test]
async fn return_ticket_succeeds_with_creator() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    assert!(board.return_ticket("abc123", None).await.is_ok());
}

#[tokio::test]
async fn return_ticket_with_comment_succeeds() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    assert!(
        board
            .return_ticket("abc123", Some("need more context"))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn return_ticket_no_creator_errors() {
    let board = MockBoard::with_tickets(vec![ticket_no_creator()]);
    let err = board.return_ticket("no-creator", None).await.unwrap_err();
    assert!(matches!(err, OrgaError::BackendError(_)));
    assert!(err.to_string().contains("no known creator"));
}

#[test]
fn comment_has_agent_name_field() {
    let c = Comment {
        id: "c1".into(),
        at: Utc::now(),
        who: sample_member(),
        content: "hello".into(),
        agent_name: Some("agent-1".into()),
    };
    let json = serde_json::to_string(&c).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["agent_name"], "agent-1");
}

#[test]
fn ticket_summary_json_has_last_commenter_is_agent() {
    let t = sample_ticket();
    let json = serde_json::to_string(&t.summary).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("last_commenter_is_agent").is_some());
    assert_eq!(parsed["last_commenter_is_agent"], false);
}

#[test]
fn comment_agent_name_null_for_humans() {
    let c = Comment {
        id: "c2".into(),
        at: Utc::now(),
        who: sample_member(),
        content: "human comment".into(),
        agent_name: None,
    };
    let json = serde_json::to_string(&c).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["agent_name"].is_null());
}

// ── Live integration tests (require real config) ──────────────────────────────
// Run with: cargo test -- --ignored

#[cfg(test)]
mod live {
    use orga::board::build_board;
    use orga::config::AppConfig;
    use std::sync::Arc;

    async fn load_board() -> Box<dyn orga::board::Board> {
        let config_path = AppConfig::resolve_path(None);
        let config = AppConfig::load(&config_path).expect("failed to load config");
        let logger = Arc::new(config.logger());
        build_board(&config, logger)
            .await
            .expect("failed to build board")
    }

    #[tokio::test]
    #[ignore]
    async fn live_list_teams() {
        let config_path = AppConfig::resolve_path(None);
        let config = AppConfig::load(&config_path).expect("failed to load config");
        let linear_cfg = config.linear.as_ref().expect("no [linear] config");
        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.linear.app/graphql")
            .header("Authorization", linear_cfg.api_key.as_str())
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "query": "{ teams { nodes { id name } } }" }))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        println!("{}", serde_json::to_string_pretty(&body).unwrap());
    }

    #[tokio::test]
    #[ignore]
    async fn live_whoami() {
        let board = load_board().await;
        let me = board.whoami().await.unwrap();
        assert!(!me.id.is_empty(), "id should not be empty");
        assert!(!me.username.is_empty(), "username should not be empty");
        println!("whoami: {} ({})", me.username, me.id);
    }

    #[tokio::test]
    #[ignore]
    async fn live_list_columns() {
        let board = load_board().await;
        let cols = board.list_columns().await.unwrap();
        assert!(!cols.is_empty(), "expected at least one column");
        for col in &cols {
            println!("column: {} ({})", col.name, col.id);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn live_get_ticket() {
        let board = load_board().await;
        let tickets = board.list_assigned().await.unwrap();
        assert!(
            !tickets.is_empty(),
            "need at least one assigned ticket to test get_ticket"
        );
        let id = &tickets[0].id;
        let ticket = board.get_ticket(id).await.unwrap();
        assert_eq!(ticket.summary.id, *id);
        assert!(!ticket.summary.title.is_empty());
        println!("ticket: {} — {}", ticket.summary.title, ticket.summary.url);
        println!(
            "  state: {} ({})",
            ticket.summary.list_name, ticket.summary.list_id
        );
        println!(
            "  assignees: {}",
            ticket
                .assignees
                .iter()
                .map(|m| m.username.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!("  comments: {}", ticket.comments.len());
        println!("  sub_tickets: {}", ticket.sub_tickets.len());
        for sub in &ticket.sub_tickets {
            println!("    [ ] {} ({})", sub.title, sub.id);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn live_return_ticket() {
        let board = load_board().await;
        let tickets = board.list_assigned().await.unwrap();
        assert!(
            !tickets.is_empty(),
            "need at least one assigned ticket to test return_ticket"
        );
        let id = &tickets[0].id;
        board
            .return_ticket(id, Some("returning ticket via orga live test"))
            .await
            .unwrap();
        println!("returned ticket {id}");
    }
}

use orga::memory::CompactionStore;
use orga::models::CommentCompaction;

use tempfile::TempDir;

// ── Compaction logic tests ─────────────────────────────────────────────────

fn make_comments_at(timestamps: &[&str]) -> Vec<Comment> {
    timestamps
        .iter()
        .enumerate()
        .map(|(i, ts)| Comment {
            id: format!("c{i}"),
            at: chrono::DateTime::parse_from_rfc3339(ts)
                .unwrap()
                .with_timezone(&Utc),
            who: sample_member(),
            content: format!("comment {i}"),
            agent_name: None,
        })
        .collect()
}

fn apply_compaction(ticket: &mut Ticket, rec: &orga::memory::CompactionRecord) {
    ticket.comments.retain(|c| c.at > rec.compacted_through);
    ticket.comment_compaction = Some(CommentCompaction {
        summary: rec.summary.clone(),
        compacted_through: rec.compacted_through,
        compacted_count: rec.compacted_count,
    });
}

#[test]
fn compaction_filters_comments_before_boundary() {
    let mut ticket = sample_ticket();
    ticket.comments = make_comments_at(&[
        "2024-01-01T10:00:00Z",
        "2024-02-01T10:00:00Z",
        "2024-03-01T10:00:00Z",
        "2024-04-01T10:00:00Z",
    ]);
    let boundary = chrono::DateTime::parse_from_rfc3339("2024-02-15T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("memory.db");
    let store = CompactionStore::open(&db_path).unwrap();
    store
        .set("abc123", "old discussion summarized", boundary, 2)
        .unwrap();
    let rec = store.get("abc123").unwrap().unwrap();
    apply_compaction(&mut ticket, &rec);
    assert_eq!(ticket.comments.len(), 2);
    assert!(ticket.comments.iter().all(|c| c.at > boundary));
    assert!(ticket.comment_compaction.is_some());
    assert!(!ticket.compaction_suggested);
}

#[test]
fn compaction_suggested_when_over_threshold_and_no_record() {
    let mut ticket = sample_ticket();
    ticket.comments = make_comments_at(&[
        "2024-01-01T10:00:00Z",
        "2024-01-02T10:00:00Z",
        "2024-01-03T10:00:00Z",
        "2024-01-04T10:00:00Z",
        "2024-01-05T10:00:00Z",
        "2024-01-06T10:00:00Z",
    ]);
    let threshold = 5;
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("memory.db");
    let store = CompactionStore::open(&db_path).unwrap();
    let rec = store.get("abc123").unwrap();
    assert!(rec.is_none());
    if ticket.comments.len() > threshold {
        ticket.compaction_suggested = true;
    }
    assert!(ticket.compaction_suggested);
    assert!(ticket.comment_compaction.is_none());
}

#[test]
fn compaction_suggested_not_set_when_record_exists() {
    let mut ticket = sample_ticket();
    ticket.comments = make_comments_at(&[
        "2024-01-01T10:00:00Z",
        "2024-01-02T10:00:00Z",
        "2024-01-03T10:00:00Z",
        "2024-01-04T10:00:00Z",
        "2024-01-05T10:00:00Z",
        "2024-01-06T10:00:00Z",
    ]);
    let boundary = chrono::DateTime::parse_from_rfc3339("2024-01-03T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("memory.db");
    let store = CompactionStore::open(&db_path).unwrap();
    store.set("abc123", "summary", boundary, 3).unwrap();
    let rec = store.get("abc123").unwrap().unwrap();
    apply_compaction(&mut ticket, &rec);
    assert!(!ticket.compaction_suggested);
    assert!(ticket.comment_compaction.is_some());
}

#[test]
fn compaction_suggested_not_set_when_under_threshold() {
    let mut ticket = sample_ticket();
    ticket.comments = make_comments_at(&[
        "2024-01-01T10:00:00Z",
        "2024-01-02T10:00:00Z",
        "2024-01-03T10:00:00Z",
    ]);
    let threshold = 5;
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("memory.db");
    let store = CompactionStore::open(&db_path).unwrap();
    let rec = store.get("abc123").unwrap();
    assert!(rec.is_none());
    if ticket.comments.len() > threshold {
        ticket.compaction_suggested = true;
    }
    assert!(!ticket.compaction_suggested);
}

#[test]
fn ticket_json_does_not_include_compaction_fields_when_absent() {
    let t = sample_ticket();
    let json = serde_json::to_string(&t).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("comment_compaction").is_none());
    assert!(parsed.get("compaction_suggested").is_none());
}

#[test]
fn ticket_json_includes_compaction_fields_when_set() {
    let mut t = sample_ticket();
    t.comment_compaction = Some(CommentCompaction {
        summary: "discussion resolved".into(),
        compacted_through: chrono::DateTime::parse_from_rfc3339("2024-03-01T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        compacted_count: 10,
    });
    t.compaction_suggested = false;
    let json = serde_json::to_string(&t).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("comment_compaction").is_some());
    assert_eq!(parsed["comment_compaction"]["compacted_count"], 10);
    assert_eq!(
        parsed["comment_compaction"]["summary"],
        "discussion resolved"
    );
}

#[test]
fn ticket_json_includes_compaction_suggested_when_true() {
    let mut t = sample_ticket();
    t.compaction_suggested = true;
    let json = serde_json::to_string(&t).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["compaction_suggested"], true);
}
