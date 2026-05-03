use chrono::Utc;

use orga::board::Board;
use orga::error::OrgaError;
use orga::models::{Checklist, Column, Comment, Member, Ticket, TicketSummary};

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

impl Board for MockBoard {
    fn list_assigned(&self) -> Result<Vec<TicketSummary>, OrgaError> {
        Ok(self.tickets.iter().map(|t| t.summary.clone()).collect())
    }

    fn get_ticket(&self, id: &str) -> Result<Ticket, OrgaError> {
        self.tickets
            .iter()
            .find(|t| t.summary.id == id)
            .cloned()
            .ok_or_else(|| OrgaError::NotFound(id.to_string()))
    }

    fn comment(&self, _id: &str, text: &str) -> Result<(), OrgaError> {
        if text.is_empty() {
            return Err(OrgaError::BackendError("empty comment".into()));
        }
        Ok(())
    }

    fn assign(&self, _id: &str, _username: &str) -> Result<(), OrgaError> {
        Ok(())
    }

    fn move_ticket(&self, _id: &str, _list: &str) -> Result<(), OrgaError> {
        Ok(())
    }

    fn create_sub(&self, parent_id: &str, title: &str) -> Result<Ticket, OrgaError> {
        let _parent = self.get_ticket(parent_id)?;
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
            },
            assignees: vec![],
            checklists: vec![],
            comments: vec![],
        })
    }

    fn add_checklist_item(&self, _id: &str, _text: &str) -> Result<String, OrgaError> {
        Ok("item-1".into())
    }

    fn check_item(&self, _id: &str, item_id: &str) -> Result<(), OrgaError> {
        if item_id == "missing" {
            return Err(OrgaError::NotFound(item_id.to_string()));
        }
        Ok(())
    }

    fn list_columns(&self) -> Result<Vec<Column>, OrgaError> {
        Ok(vec![
            Column { id: "list-1".into(), name: "To Do".into() },
            Column { id: "list-2".into(), name: "In Progress".into() },
        ])
    }

    fn whoami(&self) -> Result<Member, OrgaError> {
        Ok(self.whoami_member.clone())
    }

    fn return_ticket(&self, id: &str, _comment: Option<&str>) -> Result<(), OrgaError> {
        let ticket = self.get_ticket(id)?;
        ticket.summary.creator.ok_or_else(|| OrgaError::BackendError("ticket has no known creator".into()))?;
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
            creator: Some(Member {
                id: "u2".into(),
                username: "bob".into(),
                full_name: "Bob".into(),
            }),
        },
        assignees: vec![Member {
            id: "m1".into(),
            username: "agent-1".into(),
            full_name: "Agent One".into(),
        }],
        checklists: vec![Checklist {
            id: "cl1".into(),
            name: "Tasks".into(),
            items: vec![],
        }],
        comments: vec![Comment {
            id: "c1".into(),
            at: Utc::now(),
            who: sample_member(),
            content: "Please fix this ASAP.".into(),
            agent_name: None,
        }],
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
        },
        assignees: vec![],
        checklists: vec![],
        comments: vec![],
    }
}

#[test]
fn list_assigned_returns_tickets() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let summaries = board.list_assigned().unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, "abc123");
}

#[test]
fn list_assigned_empty() {
    let board = MockBoard::with_tickets(vec![]);
    let summaries = board.list_assigned().unwrap();
    assert!(summaries.is_empty());
}

#[test]
fn get_ticket_found() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let t = board.get_ticket("abc123").unwrap();
    assert_eq!(t.summary.title, "Fix login bug");
    assert_eq!(t.comments.len(), 1);
}

#[test]
fn get_ticket_not_found() {
    let board = MockBoard::with_tickets(vec![]);
    let err = board.get_ticket("nonexistent").unwrap_err();
    assert!(matches!(err, OrgaError::NotFound(_)));
}

#[test]
fn comment_empty_fails() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let err = board.comment("abc123", "").unwrap_err();
    assert!(matches!(err, OrgaError::BackendError(_)));
}

#[test]
fn create_sub_links_to_parent() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let sub = board.create_sub("abc123", "Sub-task one").unwrap();
    assert_eq!(sub.summary.title, "Sub-task one");
    assert_eq!(sub.summary.list_id, "list-1");
}

#[test]
fn add_checklist_item_returns_id() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let id = board.add_checklist_item("abc123", "step 1").unwrap();
    assert!(!id.is_empty());
}

#[test]
fn check_item_missing_errors() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let err = board.check_item("abc123", "missing").unwrap_err();
    assert!(matches!(err, OrgaError::NotFound(_)));
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
    assert!(parsed["checklists"].is_array());
}

#[test]
fn ticket_summary_json_has_no_detail_fields() {
    let t = sample_ticket();
    let json = serde_json::to_string(&t.summary).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["id"], "abc123");
    assert_eq!(parsed["title"], "Fix login bug");
    assert!(parsed.get("comments").is_none());
    assert!(parsed.get("checklists").is_none());
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
        },
        assignees: vec![],
        checklists: vec![],
        comments: vec![],
    }
}

#[test]
fn list_assigned_returns_all_tickets() {
    let board = MockBoard::with_tickets(vec![sample_ticket(), completed_ticket()]);
    let summaries = board.list_assigned().unwrap();
    assert_eq!(summaries.len(), 2);
}

#[test]
fn filter_open_tickets() {
    let board = MockBoard::with_tickets(vec![sample_ticket(), completed_ticket()]);
    let all = board.list_assigned().unwrap();
    let open: Vec<_> = all.iter().filter(|t| !t.completed).collect();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].id, "abc123");
}

#[test]
fn filter_completed_tickets() {
    let board = MockBoard::with_tickets(vec![sample_ticket(), completed_ticket()]);
    let all = board.list_assigned().unwrap();
    let done: Vec<_> = all.iter().filter(|t| t.completed).collect();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].id, "done1");
}

#[test]
fn filter_all_tickets() {
    let board = MockBoard::with_tickets(vec![sample_ticket(), completed_ticket()]);
    let all = board.list_assigned().unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn completed_ticket_json_has_completed_true() {
    let t = completed_ticket();
    let json = serde_json::to_string(&t.summary).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["completed"], true);
}

#[test]
fn whoami_returns_member() {
    let board = MockBoard::with_tickets(vec![]);
    let m = board.whoami().unwrap();
    assert_eq!(m.id, "agent-id");
    assert_eq!(m.username, "agent-1");
    assert_eq!(m.full_name, "Agent One");
}

#[test]
fn whoami_json_has_expected_fields() {
    let board = MockBoard::with_tickets(vec![]);
    let m = board.whoami().unwrap();
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

#[test]
fn return_ticket_succeeds_with_creator() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    assert!(board.return_ticket("abc123", None).is_ok());
}

#[test]
fn return_ticket_with_comment_succeeds() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    assert!(board.return_ticket("abc123", Some("need more context")).is_ok());
}

#[test]
fn return_ticket_no_creator_errors() {
    let board = MockBoard::with_tickets(vec![ticket_no_creator()]);
    let err = board.return_ticket("no-creator", None).unwrap_err();
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
