use chrono::Utc;

use orga::board::Board;
use orga::error::OrgaError;
use orga::models::{Checklist, Column, Comment, Member, Ticket};

struct MockBoard {
    tickets: Vec<Ticket>,
}

impl MockBoard {
    fn with_tickets(tickets: Vec<Ticket>) -> Self {
        Self { tickets }
    }
}

impl Board for MockBoard {
    fn list_assigned(&self) -> Result<Vec<Ticket>, OrgaError> {
        Ok(self.tickets.clone())
    }

    fn get_ticket(&self, id: &str) -> Result<Ticket, OrgaError> {
        self.tickets
            .iter()
            .find(|t| t.id == id)
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
            id: "sub-1".into(),
            title: title.into(),
            description: String::new(),
            list_id: "list-1".into(),
            list_name: "To Do".into(),
            url: "https://trello.com/c/sub-1".into(),
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
}

fn sample_ticket() -> Ticket {
    Ticket {
        id: "abc123".into(),
        title: "Fix login bug".into(),
        description: "The login page crashes on submit.".into(),
        list_id: "list-1".into(),
        list_name: "In Progress".into(),
        url: "https://trello.com/c/abc123".into(),
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
            who: Member {
                id: "u1".into(),
                username: "alice".into(),
                full_name: "Alice".into(),
            },
            content: "Please fix this ASAP.".into(),
        }],
    }
}

#[test]
fn list_assigned_returns_tickets() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let tickets = board.list_assigned().unwrap();
    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets[0].id, "abc123");
}

#[test]
fn list_assigned_empty() {
    let board = MockBoard::with_tickets(vec![]);
    let tickets = board.list_assigned().unwrap();
    assert!(tickets.is_empty());
}

#[test]
fn get_ticket_found() {
    let board = MockBoard::with_tickets(vec![sample_ticket()]);
    let t = board.get_ticket("abc123").unwrap();
    assert_eq!(t.title, "Fix login bug");
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
    assert_eq!(sub.title, "Sub-task one");
    assert_eq!(sub.list_id, "list-1");
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
    assert!(parsed["comments"].is_array());
    assert!(parsed["checklists"].is_array());
}
