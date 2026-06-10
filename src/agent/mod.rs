pub mod agents;
pub mod config;
pub mod context;
pub mod loop_runner;
pub mod skills;
pub mod tools;

use std::sync::Arc;
use std::time::Duration;

use rig_core::client::CompletionClient;
use rig_core::completion::{CompletionModel, Message};

use crate::board::build_board;
use crate::config::AppConfig;
use crate::error::OrgaError;
use crate::logging::Logger;
use crate::memory::{CompactionStore, ContextRepository, TodoStore};
use crate::workspace::WorkspaceStore;

use config::{LlmClient, build_llm_client};
use context::{SkillContext, build_context};
use skills::{SkillMeta, match_skills, scan_skills};
use tools::{ToolContext, dispatch, is_terminal_tool, all_tool_definitions, tool_definitions_for};

pub async fn run_agent(once: bool, dry_run: bool, config: &AppConfig, logger: Arc<Logger>) -> Result<(), OrgaError> {
    let llm_cfg = config.llm_config()?;
    let client = build_llm_client(llm_cfg)?;

    if once {
        run_once_with_client(&client, dry_run, config, Arc::clone(&logger)).await
    } else {
        run_daemon(&client, dry_run, config, Arc::clone(&logger)).await
    }
}

async fn run_daemon(
    client: &LlmClient,
    dry_run: bool,
    config: &AppConfig,
    logger: Arc<Logger>,
) -> Result<(), OrgaError> {
    let interval = Duration::from_secs(config.llm_config()?.poll_interval_secs());
    loop {
        if let Err(e) = run_once_with_client(client, dry_run, config, Arc::clone(&logger)).await {
            logger.error(&format!("[agent] poll cycle error: {e}"));
        }
        tokio::time::sleep(interval).await;
    }
}

async fn run_once_with_client(
    client: &LlmClient,
    dry_run: bool,
    config: &AppConfig,
    logger: Arc<Logger>,
) -> Result<(), OrgaError> {
    match client {
        LlmClient::Anthropic(c) => run_once(c, dry_run, config, logger).await,
        LlmClient::OpenAi(c) => run_once(c, dry_run, config, logger).await,
    }
}

async fn run_once<C>(
    client: &C,
    dry_run: bool,
    config: &AppConfig,
    logger: Arc<Logger>,
) -> Result<(), OrgaError>
where
    C: CompletionClient + Sync,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    let board = build_board(config, Arc::clone(&logger)).await?;

    let tickets = board.list_assigned().await?;
    let total = tickets.len();
    let actionable: Vec<_> = tickets
        .into_iter()
        .filter(|t| !t.completed && !t.last_commenter_is_agent)
        .collect();

    logger.info(&format!("[agent] {} ticket(s) assigned, {} waiting on agent", total, actionable.len()));

    if actionable.is_empty() {
        return Ok(());
    }

    logger.info(&format!("[agent] processing {} ticket(s)", actionable.len()));

    for summary in actionable {
        let ticket_id = summary.id.clone();
        let result = process_ticket(
            client,
            &ticket_id,
            dry_run,
            config,
            Arc::clone(&logger),
        )
        .await;

        if let Err(e) = result {
            logger.error(&format!("[agent] error processing ticket {ticket_id}: {e}"));
        }
    }

    Ok(())
}

async fn process_ticket<C>(
    client: &C,
    ticket_id: &str,
    dry_run: bool,
    config: &AppConfig,
    logger: Arc<Logger>,
) -> Result<(), OrgaError>
where
    C: CompletionClient + Sync,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    let llm_cfg = config.llm_config()?;
    let board = build_board(config, Arc::clone(&logger)).await?;
    let db_path = config.memory_db_path();
    let compaction_store = CompactionStore::open(&db_path)?;
    let context_repo = ContextRepository::open(&config.memory_repo_path(), &config.agent.name)?;

    let mut ticket = board.get_ticket(ticket_id).await?;

    if let Some(rec) = compaction_store.get(ticket_id)? {
        ticket.comments.retain(|c| c.at > rec.compacted_through);
        ticket.comment_compaction = Some(crate::models::CommentCompaction {
            summary: rec.summary,
            compacted_through: rec.compacted_through,
            compacted_count: rec.compacted_count,
        });
    }

    let ctx_msg = {
        let all_skills = config.skills_path().map(|path| scan_skills(&path, &logger)).unwrap_or_default();

        let skill_ctx = if !all_skills.is_empty() {
            let matched = match_skills(&all_skills, &ticket.summary, &logger);
            Some(SkillContext {
                available: all_skills.iter().map(|s| (s.name.clone(), s.description.clone())).collect(),
                active: matched.iter().map(|s| (s.name.clone(), s.body.clone())).collect(),
            })
        } else {
            None
        };

        let subagent_descs: Vec<(String, String)> = config.subagents.iter()
            .map(|s| (s.name.clone(), s.description.clone()))
            .collect();

        let agents_md = config.agents_md_path()
            .and_then(|p| std::fs::read_to_string(p).ok());

        build_context(
            &ticket,
            &context_repo,
            llm_cfg,
            config,
            skill_ctx.as_ref(),
            &subagent_descs,
            agents_md.as_deref(),
        )
    };

    let model = client.completion_model(&llm_cfg.model);

    // Choose tool set based on whether subagents are configured
    let tools = if config.subagents.is_empty() {
        all_tool_definitions()
    } else {
        let main_agent_tools = vec![
            "comment".to_string(),
            "dispatch".to_string(),
            "skip".to_string(),
            "done".to_string(),
            "compact".to_string(),
            "todos".to_string(),
            "memory_list".to_string(),
            "memory_read".to_string(),
            "memory_write".to_string(),
            "memory_search".to_string(),
        ];
        tool_definitions_for(&main_agent_tools)
    };

    let max_actions = llm_cfg.max_actions_per_ticket();

    logger.info(&format!("[agent] processing ticket {ticket_id} (max_actions={max_actions}, dry_run={dry_run})"));
    if dry_run {
        println!("[dry-run] processing ticket {ticket_id}");
    }

    // Build board, compaction store, todo store, and tool context once before
    // the loop — these are re-used across every iteration.
    let tool_board = build_board(config, Arc::clone(&logger)).await?;
    let tool_compaction = CompactionStore::open(&db_path)?;
    let tool_todos = TodoStore::open(&db_path)?;
    let tool_ctx = ToolContext {
        ticket_id: ticket_id.to_string(),
        agent_scope: "main".to_string(),
        board: tool_board,
        compaction_store: tool_compaction,
        todo_store: tool_todos,
        context_repo: context_repo.clone(),
        dry_run,
        logger: Arc::clone(&logger),
        workspace: config.workspace_base_path().map(WorkspaceStore::new),
    };
    let tool_ctx = Arc::new(tool_ctx);

    let mut history: Vec<Message> = vec![
        Message::system(ctx_msg.system.clone()),
        Message::user(ctx_msg.user.clone()),
    ];

    let dispatch_logger = Arc::clone(&logger);
    let dispatch_tool_ctx = Arc::clone(&tool_ctx);
    let dispatch_ticket_id = ticket_id.to_string();
    let dispatch_dry_run = dry_run;
    let dispatch_config: Arc<AppConfig> = Arc::new((*config).clone());
    let dispatch_ticket = Arc::new(ticket.clone());
    let (_, _last_text) = loop_runner::run_llm_loop(
        &model,
        &mut history,
        tools,
        max_actions,
        move |name, args, _choices| {
            let logger = Arc::clone(&dispatch_logger);
            let ctx = Arc::clone(&dispatch_tool_ctx);
            let ticket_id = dispatch_ticket_id.clone();
            let is_terminal = is_terminal_tool(&name);
            let name_str = name.clone();
            let dry_run_local = dispatch_dry_run;
            let cfg = Arc::clone(&dispatch_config);
            let tkt = Arc::clone(&dispatch_ticket);
            async move {
                logger.info(&format!("[agent] ticket {ticket_id}: calling tool '{name_str}' args={args}"));
                if dry_run_local {
                    println!("[dry-run] would call tool '{name_str}' with args: {args}");
                }
                let result = if name_str == "dispatch" {
                    handle_dispatch_tool(&args, &tkt, dry_run_local, client, cfg.as_ref(), Arc::clone(&logger)).await
                } else {
                    dispatch(&name_str, &args, &ctx).await
                };
                logger.debug(&format!("[agent] ticket {ticket_id}: tool '{name_str}' result={result}"));
                if is_terminal {
                    logger.info(&format!("[agent] ticket {ticket_id}: terminal tool called, ending cycle"));
                }
                (result, is_terminal)
            }
        },
    )
    .await?;

    // Inspect history to count actions and detect `done` (sleep-time trigger).
    let mut action_count = 0usize;
    let mut did_done = false;
    for msg in &history {
        match msg {
            Message::User { content } => {
                for c in content.iter() {
                    let s = serde_json::to_string(c).unwrap_or_default();
                    if s.contains("\"type\":\"toolresult\"")
                        || s.contains("\"type\":\"tool_result\"")
                        || s.contains("tool_call_id")
                    {
                        action_count += 1;
                    }
                }
            }
            Message::Assistant { content, .. } => {
                for c in content.iter() {
                    if let rig_core::completion::AssistantContent::ToolCall(tc) = c {
                        if tc.function.name == "done" {
                            did_done = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if action_count > 0 {
        logger.info(&format!("[agent] ticket {ticket_id}: took {action_count} action(s)"));
    } else {
        logger.info(&format!("[agent] ticket {ticket_id}: no actions taken"));
    }

    if did_done && !dry_run {
        let sleep_repo = ContextRepository::open(&config.memory_repo_path(), &config.agent.name);
        let sleep_ticket = ticket.clone();
        let sleep_config_threshold_files = config.defrag_file_threshold();
        let sleep_config_threshold_kb = config.defrag_size_threshold_kb();
        let sleep_logger = Arc::clone(&logger);
        if let Ok(repo) = sleep_repo {
            let sleep_client = client;
            let sleep_llm_model = llm_cfg.model.clone();
            if let Err(e) = run_sleep_time_agent(
                sleep_client,
                &sleep_llm_model,
                &sleep_ticket,
                repo,
                sleep_config_threshold_files,
                sleep_config_threshold_kb,
                &config.agent.name,
                &config.memory_repo_path(),
                Arc::clone(&sleep_logger),
            ).await {
                sleep_logger.error(&format!("[sleep-time] reflection error for {ticket_id}: {e}"));
            }
        }
    }

    Ok(())
}async fn handle_dispatch_tool<C>(
    args: &str,
    ticket: &crate::models::Ticket,
    dry_run: bool,
    client: &C,
    config: &AppConfig,
    logger: Arc<Logger>,
) -> String
where
    C: CompletionClient,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    let parsed: tools::DispatchArgs = match serde_json::from_str(args) {
        Ok(a) => a,
        Err(e) => return format!("error: invalid args: {e}"),
    };
    if dry_run {
        return format!("[dry-run] dispatch subagent '{}' with task: {} would have been executed", parsed.subagent, parsed.task);
    }
    let sub_cfg = config.subagents.iter().find(|s| s.name == parsed.subagent);
    let sub_cfg = match sub_cfg {
        Some(s) => s,
        None => return format!("error: no subagent named '{}' is configured", parsed.subagent),
    };
    run_subagent_loop(client, sub_cfg, ticket, &parsed.task, dry_run, config, logger).await
}

async fn run_subagent_loop<C>(
    client: &C,
    sub_cfg: &crate::config::SubagentConfig,
    ticket: &crate::models::Ticket,
    task: &str,
    dry_run: bool,
    config: &AppConfig,
    logger: Arc<Logger>,
) -> String
where
    C: CompletionClient,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    let llm_cfg = match config.llm_config() {
        Ok(c) => c,
        Err(e) => return format!("error: {e}"),
    };
    let db_path = config.memory_db_path();
    let context_repo = match ContextRepository::open(&config.memory_repo_path(), &config.agent.name) {
        Ok(r) => r,
        Err(e) => return format!("error opening context repo: {e}"),
    };

    // Build skill context for subagent
    let all_skills = config.skills_path().map(|path| scan_skills(&path, &logger)).unwrap_or_default();
    let skill_ctx = if !all_skills.is_empty() {
        let active: Vec<&SkillMeta> = if sub_cfg.skills.is_empty() {
            match_skills(&all_skills, &ticket.summary, &logger)
        } else {
            all_skills.iter().filter(|s| sub_cfg.skills.contains(&s.name)).collect()
        };
        Some(SkillContext {
            available: vec![],
            active: active.iter().map(|s| (s.name.clone(), s.body.clone())).collect(),
        })
    } else {
        None
    };

    let ctx_msg = context::build_subagent_context(
        sub_cfg,
        ticket,
        task,
        &context_repo,
        llm_cfg,
        skill_ctx.as_ref(),
    );

    let model_name = sub_cfg.model.as_deref().unwrap_or(&llm_cfg.model);
    let model = client.completion_model(model_name);
    let max_actions = sub_cfg.max_actions.unwrap_or_else(|| llm_cfg.max_actions_per_ticket());

    let mut tool_names = sub_cfg.tools.clone();
    if !tool_names.contains(&"return".to_string()) {
        tool_names.push("return".to_string());
    }
    if !tool_names.contains(&"todos".to_string()) {
        tool_names.push("todos".to_string());
    }
    for mem_tool in &["memory_list", "memory_read", "memory_write", "memory_search"] {
        if !tool_names.contains(&mem_tool.to_string()) {
            tool_names.push(mem_tool.to_string());
        }
    }
    let tools = tool_definitions_for(&tool_names);

    logger.info(&format!(
        "[subagent:{}] starting for ticket {} (max_actions={max_actions})",
        sub_cfg.name, ticket.summary.id
    ));

    let mut history: Vec<Message> = vec![
        Message::system(ctx_msg.system.clone()),
        Message::user(ctx_msg.user.clone()),
    ];

    // Open board, compaction store, todo store, and workspace once before the loop.
    let tool_board = match build_board(config, Arc::clone(&logger)).await {
        Ok(b) => b,
        Err(e) => return format!("error building board: {e}"),
    };
    let tool_compaction = match CompactionStore::open(&db_path) {
        Ok(c) => c,
        Err(e) => return format!("error opening compaction: {e}"),
    };
    let tool_todos = match TodoStore::open(&db_path) {
        Ok(t) => t,
        Err(e) => return format!("error opening todo store: {e}"),
    };

    let tool_ctx = ToolContext {
        ticket_id: ticket.summary.id.clone(),
        agent_scope: sub_cfg.name.clone(),
        board: tool_board,
        compaction_store: tool_compaction,
        todo_store: tool_todos,
        context_repo: context_repo.clone(),
        dry_run,
        logger: Arc::clone(&logger),
        workspace: config.workspace_base_path().map(WorkspaceStore::new),
    };
    let tool_ctx = Arc::new(tool_ctx);

    let subagent_name = sub_cfg.name.clone();
    let dispatch_logger = Arc::clone(&logger);
    let dispatch_tool_ctx = Arc::clone(&tool_ctx);
    let mut last_return_value: Option<String> = None;
    let (outcome, last_text) = loop_runner::run_llm_loop(
        &model,
        &mut history,
        tools,
        max_actions,
        move |name, args, _choices| {
            let logger = Arc::clone(&dispatch_logger);
            let ctx = Arc::clone(&dispatch_tool_ctx);
            let is_return = name == "return";
            let sub_name = subagent_name.clone();
            async move {
                logger.info(&format!("[subagent:{sub_name}] calling tool '{name}' args={args}"));
                let result = dispatch(&name, &args, &ctx).await;
                logger.debug(&format!("[subagent:{sub_name}] tool '{name}' result={result}"));
                (result, is_return)
            }
        },
    )
    .await
    .unwrap_or_else(|e| (loop_runner::LoopOutcome::NoToolCalls, format!("error: {e}")));

    match outcome {
        loop_runner::LoopOutcome::Terminal => {
            // The last tool call in the terminal turn was `return`; recover the
            // return value from the just-appended history entry.
            if let Some(Message::User { content }) = history.last() {
                if let Some(c) = content.iter().next() {
                    let s = serde_json::to_string(c).unwrap_or_default();
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&s) {
                        if let Some(text) = parsed.get("text").and_then(|v| v.as_str()) {
                            last_return_value = Some(text.to_string());
                        }
                    }
                }
            }
            logger.info(&format!("[subagent:{}] return called, finishing", sub_cfg.name));
            last_return_value.unwrap_or(last_text)
        }
        loop_runner::LoopOutcome::CapReached => {
            logger.info(&format!("[subagent:{}] hit action cap without returning", sub_cfg.name));
            "error: subagent hit action cap without returning a result".to_string()
        }
        loop_runner::LoopOutcome::NoToolCalls => {
            logger.info(&format!("[subagent:{}] no tool calls, returning last text", sub_cfg.name));
            last_text
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_sleep_time_agent<C>(
    client: &C,
    model_name: &str,
    ticket: &crate::models::Ticket,
    context_repo: ContextRepository,
    defrag_file_threshold: usize,
    defrag_size_threshold_kb: u64,
    agent_name: &str,
    memory_repo_path: &std::path::Path,
    logger: Arc<Logger>,
) -> Result<(), OrgaError>
where
    C: CompletionClient + Sync,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    logger.info(&format!("[sleep-time] reflecting on ticket {}", ticket.summary.id));

    let entries = context_repo.list().unwrap_or_default();
    let tree_index = if entries.is_empty() {
        "(empty)".to_string()
    } else {
        entries.iter()
            .map(|e| if e.description.is_empty() { e.path.clone() } else { format!("{} — {}", e.path, e.description) })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let system = format!(
        "You are a memory reflection agent. Your job is to review a completed ticket and persist \
any cross-ticket-valuable knowledge into the context repository. Focus on: recurring themes, \
architectural patterns, team conventions, people preferences, and recurring problems.\n\n\
Do NOT save ticket-specific facts. Only save information that would help on FUTURE tickets.\n\n\
Available tools: memory_list, memory_read, memory_write.\n\
Use memory_write to create or update topic files with YAML frontmatter (description: field).\n\
When done, stop — do not call any other tools.\n\n\
Current repository index:\n{tree_index}"
    );

    let user = format!(
        "Ticket just completed: {}\n\nDescription: {}\n\n## Comments\n{}\n\nReflect and persist any valuable cross-ticket learnings.",
        ticket.summary.title,
        ticket.summary.description,
        ticket.comments.iter()
            .map(|c| format!("@{}: {}", c.who.username, c.content))
            .collect::<Vec<_>>()
            .join("\n---\n")
    );

    let model = client.completion_model(model_name);
    let sleep_tools = tool_definitions_for(&[
        "memory_list".to_string(),
        "memory_read".to_string(),
        "memory_write".to_string(),
    ]);

    let mut history: Vec<Message> = vec![
        Message::system(system),
        Message::user(user),
    ];

    let dispatch_logger = Arc::clone(&logger);
    let (outcome, _last_text) = loop_runner::run_llm_loop(
        &model,
        &mut history,
        sleep_tools,
        10,
        move |name, args, _choices| {
            let logger = Arc::clone(&dispatch_logger);
            let repo_clone = context_repo.clone();
            async move {
                let ctx = tools::SleepToolContext {
                    context_repo: repo_clone,
                    logger: Arc::clone(&logger),
                };
                let result = tools::dispatch_sleep_tool(&name, &args, &ctx).await;
                logger.debug(&format!("[sleep-time] tool '{name}' result={result}"));
                (result, false)
            }
        },
    )
    .await?;

    logger.debug(&format!("[sleep-time] loop ended with {:?}", outcome));

    // Check thresholds and run defrag if needed
    let fresh_repo = ContextRepository::open(memory_repo_path, agent_name)?;
    if let Ok(stats) = fresh_repo.repo_stats() {
        if stats.file_count >= defrag_file_threshold || stats.total_size_kb >= defrag_size_threshold_kb {
            logger.info("[sleep-time] threshold exceeded, running defrag");
            if let Err(e) = run_defrag_agent(client, model_name, memory_repo_path, agent_name, Arc::clone(&logger)).await {
                logger.error(&format!("[sleep-time] defrag error: {e}"));
            }
        }
    }

    logger.info(&format!("[sleep-time] reflection complete for {}", ticket.summary.id));
    Ok(())
}

async fn run_defrag_agent<C>(
    client: &C,
    model_name: &str,
    memory_repo_path: &std::path::Path,
    agent_name: &str,
    logger: Arc<Logger>,
) -> Result<(), OrgaError>
where
    C: CompletionClient + Sync,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    logger.info("[defrag] starting defragmentation pass");

    let repo = ContextRepository::open(memory_repo_path, agent_name)?;
    let entries = repo.list().unwrap_or_default();
    let tree = entries.iter()
        .map(|e| if e.description.is_empty() { e.path.clone() } else { format!("{} — {}", e.path, e.description) })
        .collect::<Vec<_>>()
        .join("\n");

    let system = format!(
        "You are a memory cleanup agent. Your job is to reduce clutter in the context repository.\n\n\
Your tasks:\n\
1. Split files that cover multiple distinct topics (aim for 15-50 lines per file)\n\
2. Merge files with heavily overlapping content — write the merged result, then delete the originals using memory_delete\n\
3. Delete files that are redundant (all their content exists in other files)\n\n\
Rules:\n\
- Do NOT rename folders or restructure the directory hierarchy\n\
- Do NOT update frontmatter descriptions unless directly related to a merge/split\n\
- After merging two files into one, always delete the originals with memory_delete\n\
- If memory_delete returns an error (content not covered elsewhere), do not delete that file\n\n\
Available tools: memory_list, memory_read, memory_write, memory_delete.\n\
When done cleaning up, stop.\n\n\
Current repository:\n{tree}"
    );

    let model = client.completion_model(model_name);
    let defrag_tools = tools::defrag_tool_definitions();

    let mut history: Vec<Message> = vec![
        Message::system(system),
        Message::user("Please clean up the context repository now.".to_string()),
    ];

    let dispatch_logger = Arc::clone(&logger);
    let (outcome, _last_text) = loop_runner::run_llm_loop(
        &model,
        &mut history,
        defrag_tools,
        20,
        move |name, args, _choices| {
            let logger = Arc::clone(&dispatch_logger);
            let repo_clone = repo.clone();
            async move {
                let ctx = tools::SleepToolContext {
                    context_repo: repo_clone,
                    logger: Arc::clone(&logger),
                };
                let result = tools::dispatch_sleep_tool(&name, &args, &ctx).await;
                logger.debug(&format!("[defrag] tool '{name}' result={result}"));
                (result, false)
            }
        },
    )
    .await?;

    logger.debug(&format!("[defrag] loop ended with {:?}", outcome));
    logger.info("[defrag] defragmentation complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use rig_core::completion::Message;

    fn history_entry_for(tool_name: &str, tool_call_id: &str, result: &str) -> Message {
        if tool_name == "dispatch" {
            Message::user(result)
        } else {
            Message::tool_result(tool_call_id, result)
        }
    }

    fn is_tool_result_message(msg: &Message) -> bool {
        if let Message::User { content } = msg {
            content.iter().any(|c| {
                let s = serde_json::to_string(c).unwrap_or_default();
                s.contains("\"type\":\"toolresult\"") || s.contains("\"type\":\"tool_result\"") || s.contains("tool_call_id")
            })
        } else {
            false
        }
    }

    fn is_plain_text_user_message(msg: &Message) -> bool {
        if let Message::User { content } = msg {
            content.iter().any(|c| {
                let s = serde_json::to_string(c).unwrap_or_default();
                s.contains("\"type\":\"text\"")
            })
        } else {
            false
        }
    }

    #[test]
    fn dispatch_result_is_plain_user_message_not_tool_result() {
        let msg = history_entry_for("dispatch", "call_main_abc", "subagent finished");
        assert!(is_plain_text_user_message(&msg), "dispatch should produce a plain text user message");
        assert!(!is_tool_result_message(&msg), "dispatch must NOT produce a tool_result message");
    }

    #[test]
    fn comment_result_is_tool_result_message() {
        let msg = history_entry_for("comment", "call_main_xyz", "comment posted");
        assert!(is_tool_result_message(&msg), "comment should produce a tool_result message");
    }

    #[test]
    fn done_result_is_tool_result_message() {
        let msg = history_entry_for("done", "call_main_done", "done");
        assert!(is_tool_result_message(&msg), "done should produce a tool_result message");
    }
}
