use chrono::Local;
use crate::config::{AppConfig, LlmConfig, SubagentConfig};
use crate::memory::ContextRepository;
use crate::models::Ticket;

const MAIN_AGENT_SYSTEM_PROMPT: &str = include_str!("prompts/main_agent.md");
const DISPATCHER_SYSTEM_PROMPT: &str = include_str!("prompts/dispatcher.md");

pub struct SkillContext {
    pub available: Vec<(String, String)>,
    pub active: Vec<(String, String)>,
}

pub struct TicketContext {
    pub system: String,
    pub user: String,
}

pub fn build_context(
    ticket: &Ticket,
    context_repo: &ContextRepository,
    llm_cfg: &LlmConfig,
    app_cfg: &AppConfig,
    skill_ctx: Option<&SkillContext>,
    subagents: &[(String, String)],
    agents_md: Option<&str>,
) -> TicketContext {
    let system = build_system_prompt(ticket, context_repo, app_cfg, skill_ctx, subagents, agents_md);
    let user = build_user_message(ticket, llm_cfg);
    TicketContext { system, user }
}

pub fn build_subagent_context(
    subagent_cfg: &SubagentConfig,
    ticket: &Ticket,
    task: &str,
    context_repo: &ContextRepository,
    llm_cfg: &LlmConfig,
    skill_ctx: Option<&SkillContext>,
) -> TicketContext {
    let system = build_subagent_system_prompt(subagent_cfg, context_repo, skill_ctx);
    let mut user = build_user_message(ticket, llm_cfg);
    user.push_str(&format!("\n\n## Your Task\n{task}"));
    TicketContext { system, user }
}

fn build_subagent_system_prompt(subagent_cfg: &SubagentConfig, context_repo: &ContextRepository, skill_ctx: Option<&SkillContext>) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(ref prompt) = subagent_cfg.system_prompt {
        parts.push(prompt.clone());
    }

    let tools_list = subagent_cfg.tools.join(", ");
    parts.push(format!(
        "You are a specialized subagent named '{}'. Your role: {}\n\
\n\
Available tools: {}, return.\n\
\n\
Use `return(result)` when you have completed your task — pass back a concise result summary.\n\
Do NOT call `comment`, `done`, or `skip` unless they are in your available tools.",
        subagent_cfg.name, subagent_cfg.description, tools_list
    ));

    parts.push(build_context_repo_section(context_repo));

    if let Some(ctx) = skill_ctx
        && !ctx.active.is_empty()
    {
        let mut section = "\n## Active Skills".to_string();
        for (name, body) in &ctx.active {
            section.push_str(&format!("\n### {name}\n{body}"));
        }
        parts.push(section);
    }

    parts.join("\n")
}

fn build_system_prompt(_ticket: &Ticket, context_repo: &ContextRepository, app_cfg: &AppConfig, skill_ctx: Option<&SkillContext>, subagents: &[(String, String)], agents_md: Option<&str>) -> String {
    let mut parts: Vec<String> = Vec::new();

    if subagents.is_empty() {
        parts.push(
            MAIN_AGENT_SYSTEM_PROMPT
                .replace("{agent_name}", &app_cfg.agent.name)
                .replace("{tools}", &crate::agent::tools::MAIN_TOOLS.join(", ")),
        );
    } else {
        parts.push(
            DISPATCHER_SYSTEM_PROMPT
                .replace("{agent_name}", &app_cfg.agent.name)
                .replace("{tools}", &crate::agent::tools::MAIN_TOOLS.join(", ")),
        );

        let mut section = "\n## Available Subagents".to_string();
        for (name, desc) in subagents {
            section.push_str(&format!("\n- **{name}**: {desc}"));
        }
        parts.push(section);
    }

    if let Some(ctx) = skill_ctx
        && !ctx.available.is_empty() {
            let mut section = "\n## Available Skills".to_string();
            for (name, desc) in &ctx.available {
                section.push_str(&format!("\n- **{name}**: {desc}"));
            }
            parts.push(section);
        }

    if let Some(md) = agents_md {
        parts.push(format!("\n## Agent Instructions\n{}", md));
    }

    if let Some(ctx) = skill_ctx
        && !ctx.active.is_empty() {
            let mut section = "\n## Active Skills".to_string();
            for (name, body) in &ctx.active {
                section.push_str(&format!("\n### {name}\n{body}"));
            }
            parts.push(section);
        }

    parts.push(build_context_repo_section(context_repo));

    parts.join("\n")
}

fn build_context_repo_section(context_repo: &ContextRepository) -> String {
    let mut section = String::from("\n## Context Repository");

    let entries = context_repo.list().unwrap_or_default();
    if entries.is_empty() {
        section.push_str("\n*(empty — no memory files yet)*");
    } else {
        for entry in &entries {
            if entry.description.is_empty() {
                section.push_str(&format!("\n- {}", entry.path));
            } else {
                section.push_str(&format!("\n- {} — {}", entry.path, entry.description));
            }
        }
    }

    let system_files = context_repo.system_files().unwrap_or_default();
    if !system_files.is_empty() {
        section.push_str("\n\n## Context Repository (pinned)");
        for (path, content) in &system_files {
            section.push_str(&format!("\n\n### {path}\n{content}"));
        }
    }

    section
}

fn build_user_message(
    ticket: &Ticket,
    _llm_cfg: &LlmConfig,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!("# Ticket: {}", ticket.summary.title));
    parts.push(format!("**ID:** {}", ticket.summary.id));
    parts.push(format!("**Column:** {}", ticket.summary.list_name));
    parts.push(format!("**URL:** {}", ticket.summary.url));
    parts.push(format!("**Today's date:** {}", Local::now().format("%Y-%m-%d")));

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

    if !ticket.sub_tickets.is_empty() {
        parts.push("\n## Sub-tickets".to_string());
        for sub in &ticket.sub_tickets {
            let mark = if sub.completed { "[x]" } else { "[ ]" };
            parts.push(format!("- {} {} (id: {}) {}", mark, sub.title, sub.id, sub.url));
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

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentConfig, AppConfig, BoardConfig, LlmConfig};
    use crate::models::{CommentCompaction, Member, Ticket, TicketSummary};
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
                labels: vec![],
            },
            assignees: vec![],
            sub_tickets: vec![],
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
        }
    }

    fn make_app_cfg() -> AppConfig {
        AppConfig {
            agent: AgentConfig { name: "bot-1".to_string() },
            board: BoardConfig { backend: "trello".to_string() },
            trello: None,
            linear: None,
            memory: None,
            logging: None,
            llm: None,
            comment_compaction_threshold: None,
            skills: None,
            workspace: None,
            subagents: vec![],
            metrics: None,
        }
    }

    fn open_repo(dir: &std::path::Path) -> ContextRepository {
        ContextRepository::open(&dir.join("memory"), "bot-1").unwrap()
    }

    #[test]
    fn context_includes_ticket_title() {
        let ticket = make_ticket("Fix the bug");
        let dir = tempdir().unwrap();
        let repo = open_repo(dir.path());
        let ctx = build_context(&ticket, &repo, &make_llm_cfg(), &make_app_cfg(), None, &[], None);
        assert!(ctx.user.contains("Test Ticket"));
        assert!(ctx.user.contains("Fix the bug"));
    }

    #[test]
    fn context_system_includes_agent_name() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let repo = open_repo(dir.path());
        let ctx = build_context(&ticket, &repo, &make_llm_cfg(), &make_app_cfg(), None, &[], None);
        assert!(ctx.system.contains("bot-1"));
    }

    #[test]
    fn context_system_includes_repo_section() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let repo = open_repo(dir.path());
        let ctx = build_context(&ticket, &repo, &make_llm_cfg(), &make_app_cfg(), None, &[], None);
        assert!(ctx.system.contains("## Context Repository"));
    }

    #[test]
    fn context_system_injects_system_files() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let repo = open_repo(dir.path());
        let ctx = build_context(&ticket, &repo, &make_llm_cfg(), &make_app_cfg(), None, &[], None);
        assert!(ctx.system.contains("## Context Repository (pinned)"));
        assert!(ctx.system.contains("system/overview.md"));
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
        let repo = open_repo(dir.path());
        let ctx = build_context(&ticket, &repo, &make_llm_cfg(), &make_app_cfg(), None, &[], None);
        assert!(ctx.user.contains("first 10 comments: auth work"));
    }

    #[test]
    fn system_prompt_includes_available_skills() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let repo = open_repo(dir.path());
        let skill_ctx = SkillContext {
            available: vec![
                ("code-review".to_string(), "Reviews code.".to_string()),
                ("security".to_string(), "Audits security.".to_string()),
            ],
            active: vec![],
        };
        let ctx = build_context(&ticket, &repo, &make_llm_cfg(), &make_app_cfg(), Some(&skill_ctx), &[], None);
        assert!(ctx.system.contains("## Available Skills"));
        assert!(ctx.system.contains("**code-review**"));
        assert!(ctx.system.contains("**security**"));
    }

    #[test]
    fn system_prompt_includes_active_skills_body() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let repo = open_repo(dir.path());
        let skill_ctx = SkillContext {
            available: vec![("code-review".to_string(), "Reviews code.".to_string())],
            active: vec![("code-review".to_string(), "Follow these steps to review code.".to_string())],
        };
        let ctx = build_context(&ticket, &repo, &make_llm_cfg(), &make_app_cfg(), Some(&skill_ctx), &[], None);
        assert!(ctx.system.contains("## Active Skills"));
        assert!(ctx.system.contains("### code-review"));
        assert!(ctx.system.contains("Follow these steps to review code."));
    }

    #[test]
    fn system_prompt_no_skills_sections_when_no_context() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let repo = open_repo(dir.path());
        let ctx = build_context(&ticket, &repo, &make_llm_cfg(), &make_app_cfg(), None, &[], None);
        assert!(!ctx.system.contains("## Available Skills"));
        assert!(!ctx.system.contains("## Active Skills"));
    }

    #[test]
    fn system_prompt_no_active_section_when_none_matched() {
        let ticket = make_ticket("");
        let dir = tempdir().unwrap();
        let repo = open_repo(dir.path());
        let skill_ctx = SkillContext {
            available: vec![("s".to_string(), "desc".to_string())],
            active: vec![],
        };
        let ctx = build_context(&ticket, &repo, &make_llm_cfg(), &make_app_cfg(), Some(&skill_ctx), &[], None);
        assert!(ctx.system.contains("## Available Skills"));
        assert!(!ctx.system.contains("## Active Skills"));
    }
}
