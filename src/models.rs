use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentCompaction {
    pub summary: String,
    pub compacted_through: DateTime<Utc>,
    pub compacted_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMeta {
    pub ticket_id: String,
    pub agent_name: String,
    pub name: String,
    pub committed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    #[serde(flatten)]
    pub meta: ArtifactMeta,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub id: String,
    pub username: String,
    pub full_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub at: DateTime<Utc>,
    pub who: Member,
    pub content: String,
    pub agent_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketSummary {
    pub id: String,
    pub title: String,
    pub description: String,
    pub list_id: String,
    pub list_name: String,
    pub url: String,
    pub completed: bool,
    pub creator: Option<Member>,
    pub last_commenter_is_agent: bool,
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    #[serde(flatten)]
    pub summary: TicketSummary,
    pub assignees: Vec<Member>,
    pub sub_tickets: Vec<TicketSummary>,
    pub comments: Vec<Comment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_compaction: Option<CommentCompaction>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub compaction_suggested: bool,
}
