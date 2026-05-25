use crate::artifact::ArtifactStore;
use crate::config::{AppConfig, LlmConfig};
use crate::memory::MemoryStore;
use crate::models::Ticket;

pub struct TicketContext {
    pub system: String,
    pub user: String,
}

pub fn build_context(
    ticket: &Ticket,
    memory_store: &MemoryStore,
    artifact_store: Option<&Box<dyn ArtifactStore>>,
    llm_cfg: &LlmConfig,
    app_cfg: &AppConfig,
) -> TicketContext {
    let system = build_system_prompt(ticket, app_cfg);
    let user = build_user_message(ticket, memory_store, artifact_store, llm_cfg);
    TicketContext { system, user }
}

fn build_system_prompt(ticket: &Ticket, app_cfg: &AppConfig) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!(
        "You are an AI agent named '{}' operating on a kanban board. \
You communicate with teammates exclusively through ticket comments, artifact files, \
checklists, and board actions. You are a first-class board member alongside humans.\n\
\n\
Available tools: comment, move_ticket, assign, create_sub, add_checklist_item, check_item, \
set_memory, commit_artifact, get_artifact, compact, done, skip.\n\
\n\
Use `done(comment?)` when you have completed work on a ticket — this returns it to the creator.\n\
Use `skip()` if the ticket is not actionable right now.",
        app_cfg.agent.name
    ));

    if let Some(prompt) = app_cfg.workflow_prompt(&ticket.summary.list_name) {
        parts.push(format!("\n## Column Instructions\n{}", prompt));
    }

    parts.join("\n")
}

fn build_user_message(
    ticket: &Ticket,
    memory_store: &MemoryStore,
    artifact_store: Option<&Box<dyn ArtifactStore>>,
    llm_cfg: &LlmConfig,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!("# Ticket: {}", ticket.summary.title));
    parts.push(format!("**ID:** {}", ticket.summary.id));
    parts.push(format!("**Column:** {}", ticket.summary.list_name));
    parts.push(format!("**URL:** {}", ticket.summary.url));

    if let Some(ref creator) = ticket.summary.creator {
        parts.push(format!("**Creator:** @{}", creator.username));
    }

    if !ticket.assignees.is_empty() {
        let names: Vec<&str> = ticket.assignees.iter().map(|m| m.username.as_str()).collect();
        parts.push(format!("**Assignees:** {}", names.join(", ")));
    }

    if !ticket.summary.description.is_empty() {
        parts.push(format!("\n## Description\n{}", ticket.summary.description));
    }

    if !ticket.checklists.is_empty() {
        parts.push("\n## Checklists".to_string());
        for cl in &ticket.checklists {
            parts.push(format!("### {}", cl.name));
            for item in &cl.items {
                let mark = if item.complete { "[x]" } else { "[ ]" };
                parts.push(format!("- {} {} (id: {})", mark, item.text, item.id));
            }
        }
    }

    if ticket.comment_compaction.is_some() || !ticket.comments.is_empty() {
        parts.push("\n## Comments".to_string());
        if let Some(ref cc) = ticket.comment_compaction {
            parts.push(format!(
                "*[Compacted: {} comments through {}]*\n*Summary: {}*",
                cc.compacted_count,
                cc.compacted_through.format("%Y-%m-%d %H:%M"),
                cc.summary
            ));
            if !ticket.comments.is_empty() {
                parts.push("---".to_string());
            }
        }
        for c in &ticket.comments {
            parts.push(format!(
                "**@{}** at {}:\n{}",
                c.who.username,
                c.at.format("%Y-%m-%d %H:%M"),
                c.content
            ));
        }
    }

    if let Ok(Some(mem)) = memory_store.get(&ticket.summary.id) {
        parts.push(format!("\n## Agent Memory (private)\n{}", mem.context));
    }

    if let Some(store) = artifact_store {
        if let Ok(metas) = store.list(&ticket.summary.id) {
            if !metas.is_empty() {
                parts.push("\n## Artifacts".to_string());
                let cap = llm_cfg.max_artifact_inline_bytes();
                for meta in &metas {

                    let inlined = if let Ok(Some(artifact)) = store.get(&ticket.summary.id, &meta.name) {
                        let bytes = artifact.content.len();
                        if bytes <= cap {
                            Some(artifact.content)
                        } else {
                            parts.push(format!(
                                "### {} (by @{}, {})\n*Content too large to inline ({} bytes > {} cap). Call `get_artifact(\"{}\")` to read.*",
                                meta.name,
                                meta.agent_name,
                                meta.committed_at.format("%Y-%m-%d %H:%M"),
                                bytes,
                                cap,
                                meta.name,
                            ));
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(content) = inlined {
                        parts.push(format!(
                            "### {} (by @{}, {})\n```\n{}\n```",
                            meta.name,
                            meta.agent_name,
                            meta.committed_at.format("%Y-%m-%d %H:%M"),
                            content,
                        ));
                    }
                }
            }
        }
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentConfig, AppConfig, BoardConfig, LlmConfig};
    use crate::models::{
        Artifact, ArtifactMeta, CommentCompaction, Member, Ticket, TicketSummary,
    };
    use chrono::Utc;
    use tempfile::tempdir;

    fn make_ticket(description: &str) -> Ticket {
        Ticket {
            summary: TicketSummary {
                id: "T-1".to_string(),
                title: "Test Ticket".to_string(),
                description: description.to_string(),
                list_id: "col-1".to_string(),
                list_name: "In Progress".to_string(),
                url: "https://example.com/T-1".to_string(),
                completed: false,
                creator: Some(Member { id: "u1".to_string(), username: "alice".to_string(), full_name: "Alice".to_string() }),
                last_commenter_is_agent: false,
            },
            assignees: vec![],
            checklists: vec![],
            comments: vec![],
            comment_compaction: None,
            compaction_suggested: false,
        }
    }

    fn make_llm_cfg() -> LlmConfig {
        LlmConfig {
            provider: "anthropic".to_string(),
            api_key: "test".to_string(),
            model: "test".to_string(),
            endpoint: None,
            poll_interval_secs: None,
            max_actions_per_ticket: None,
            max_artifact_inline_bytes: Some(8192),
        }
    }

    fn make_app_cfg() -> AppConfig {
        AppConfig {
            agent: AgentConfig { name: "bot-1".to_string() },
            board: BoardConfig { backend: "trello".to_string() },
            trello: None,
            linear: None,
            memory: None,
            artifact: None,
            logging: None,
            llm: None,
            workflow: vec![],
            comment_compaction_threshold: None,
        }
    }

    fn open_memory(dir: &std::path::Path) -> MemoryStore {
        MemoryStore::open(&dir.join("mem.db")).unwrap()
    }

    #[test]
    fn context_includes_ticket_title() {
        let ticket = make_ticket("Fix the bug");
        let dir = tempdir().unwrap();
        let mem = open_memory(dir.path());
        let ctx = build_context(&ticket, &mem, None, &make_llm_cfg(), &make_app_cfg());
        assert!(ctx.user.contains("Test Ticket"));
        assert!(ctx.user.contains("Fix the bug"));
    }

    #[test]
    fn context_system_includes_agent_name() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let mem = open_memory(dir.path());
        let ctx = build_context(&ticket, &mem, None, &make_llm_cfg(), &make_app_cfg());
        assert!(ctx.system.contains("bot-1"));
    }

    #[test]
    fn context_includes_memory_when_present() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let mem = open_memory(dir.path());
        mem.set("T-1", "remember this context").unwrap();
        let ctx = build_context(&ticket, &mem, None, &make_llm_cfg(), &make_app_cfg());
        assert!(ctx.user.contains("remember this context"));
    }

    #[test]
    fn context_omits_memory_section_when_absent() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let mem = open_memory(dir.path());
        let ctx = build_context(&ticket, &mem, None, &make_llm_cfg(), &make_app_cfg());
        assert!(!ctx.user.contains("Agent Memory"));
    }

    #[test]
    fn context_includes_compaction_summary() {
        let mut ticket = make_ticket("");
        ticket.comment_compaction = Some(CommentCompaction {
            summary: "first 10 comments: auth work".to_string(),
            compacted_through: Utc::now(),
            compacted_count: 10,
        });
        let dir = tempdir().unwrap();
        let mem = open_memory(dir.path());
        let ctx = build_context(&ticket, &mem, None, &make_llm_cfg(), &make_app_cfg());
        assert!(ctx.user.contains("first 10 comments: auth work"));
    }

    struct InlineArtifactStore {
        content: String,
    }

    impl ArtifactStore for InlineArtifactStore {
        fn commit(&self, _ticket_id: &str, _name: &str, _content: &[u8]) -> Result<ArtifactMeta, crate::error::OrgaError> {
            Err(crate::error::OrgaError::BackendError("mock".into()))
        }
        fn get(&self, _ticket_id: &str, name: &str) -> Result<Option<Artifact>, crate::error::OrgaError> {
            Ok(Some(Artifact {
                meta: ArtifactMeta {
                    ticket_id: "T-1".to_string(),
                    agent_name: "bot".to_string(),
                    name: name.to_string(),
                    committed_at: Utc::now(),
                },
                content: self.content.clone(),
            }))
        }
        fn list(&self, _ticket_id: &str) -> Result<Vec<ArtifactMeta>, crate::error::OrgaError> {
            Ok(vec![ArtifactMeta {
                ticket_id: "T-1".to_string(),
                agent_name: "bot".to_string(),
                name: "report.md".to_string(),
                committed_at: Utc::now(),
            }])
        }
    }

    #[test]
    fn artifact_inlined_below_cap() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let mem = open_memory(dir.path());
        let store: Box<dyn ArtifactStore> = Box::new(InlineArtifactStore {
            content: "small content".to_string(),
        });
        let ctx = build_context(&ticket, &mem, Some(&store), &make_llm_cfg(), &make_app_cfg());
        assert!(ctx.user.contains("small content"));
        assert!(!ctx.user.contains("too large"));
    }

    #[test]
    fn artifact_metadata_only_above_cap() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let mem = open_memory(dir.path());
        let big = "x".repeat(9000);
        let store: Box<dyn ArtifactStore> = Box::new(InlineArtifactStore { content: big });
        let mut llm_cfg = make_llm_cfg();
        llm_cfg.max_artifact_inline_bytes = Some(100);
        let ctx = build_context(&ticket, &mem, Some(&store), &llm_cfg, &make_app_cfg());
        assert!(ctx.user.contains("get_artifact"));
        assert!(!ctx.user.contains("xxxxxxxxxx"));
    }
}
