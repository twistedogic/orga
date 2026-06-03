pub mod config;
pub mod context;
pub mod skills;
pub mod tools;

use std::sync::Arc;
use std::time::Duration;

use rig_core::client::CompletionClient;
use rig_core::completion::{AssistantContent, CompletionModel, CompletionRequest, Message};

use crate::board::build_board;
use crate::config::AppConfig;
use crate::error::OrgaError;
use crate::logging::Logger;
use crate::memory::{CompactionStore, MemoryStore, TodoStore};
use crate::workspace::WorkspaceStore;

use config::{LlmClient, build_llm_client};
use context::{SkillContext, build_context};
use skills::{SkillMeta, match_skills, scan_skills};
use tools::{ToolContext, dispatch, is_terminal_tool, tool_definitions, tool_definitions_for};

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
    let memory_store = MemoryStore::open(&db_path)?;
    let compaction_store = CompactionStore::open(&db_path)?;

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

        build_context(
            &ticket,
            &memory_store,
            llm_cfg,
            config,
            skill_ctx.as_ref(),
            &subagent_descs,
        )
    };

    let model = client.completion_model(&llm_cfg.model);

    // Choose tool set based on whether subagents are configured
    let tools = if config.subagents.is_empty() {
        tool_definitions()
    } else {
        let main_agent_tools = vec![
            "comment".to_string(),
            "dispatch".to_string(),
            "skip".to_string(),
            "done".to_string(),
            "set_memory".to_string(),
            "compact".to_string(),
            "todos".to_string(),
        ];
        tool_definitions_for(&main_agent_tools)
    };

    let max_actions = llm_cfg.max_actions_per_ticket();

    logger.info(&format!("[agent] processing ticket {ticket_id} (max_actions={max_actions}, dry_run={dry_run})"));
    if dry_run {
        println!("[dry-run] processing ticket {ticket_id}");
    }

    let mut action_count = 0usize;
    let mut history: Vec<Message> = vec![
        Message::system(ctx_msg.system.clone()),
        Message::user(ctx_msg.user.clone()),
    ];

    loop {
        if action_count >= max_actions {
            logger.info(&format!("[agent] ticket {ticket_id}: max actions cap ({max_actions}) reached"));
            break;
        }

        let req = CompletionRequest {
            model: None,
            preamble: None,
            chat_history: rig_core::one_or_many::OneOrMany::many(history.clone())
                .expect("history is non-empty"),
            documents: vec![],
            tools: tools.clone(),
            temperature: None,
            max_tokens: None,
            tool_choice: None,
            additional_params: None,
            output_schema: None,
        };

        let response = model.completion(req).await.map_err(|e| {
            OrgaError::BackendError(format!("LLM completion error for {ticket_id}: {e}"))
        })?;

        let choices: Vec<AssistantContent> = response.choice.into_iter().collect();

        let tool_calls: Vec<_> = choices
            .iter()
            .filter_map(|c| if let AssistantContent::ToolCall(tc) = c { Some(tc.clone()) } else { None })
            .collect();

        if let Ok(content) = rig_core::one_or_many::OneOrMany::many(choices) {
            history.push(Message::Assistant { id: None, content });
        }

        if tool_calls.is_empty() {
            logger.info(&format!("[agent] ticket {ticket_id}: LLM returned no tool calls, ending cycle"));
            break;
        }

        let tool_board = build_board(config, Arc::clone(&logger)).await?;
        let tool_memory = MemoryStore::open(&db_path)?;
        let tool_compaction = CompactionStore::open(&db_path)?;
        let tool_todos = TodoStore::open(&db_path)?;

        let tool_ctx = ToolContext {
            ticket_id: ticket_id.to_string(),
            agent_scope: "main".to_string(),
            board: tool_board,
            memory_store: tool_memory,
            compaction_store: tool_compaction,
            todo_store: tool_todos,
            dry_run,
            logger: Arc::clone(&logger),
            workspace: config.workspace_base_path().map(WorkspaceStore::new),
        };

        let mut terminal = false;
        for tc in &tool_calls {
            let name = &tc.function.name;
            let args = tc.function.arguments.to_string();

            logger.info(&format!("[agent] ticket {ticket_id}: calling tool '{name}' args={args}"));
            if dry_run {
                println!("[dry-run] would call tool '{name}' with args: {args}");
            }

            let result = if name == "dispatch" {
                handle_dispatch_tool(&args, &ticket, dry_run, client, config, Arc::clone(&logger)).await
            } else {
                dispatch(name, &args, &tool_ctx).await
            };

            logger.debug(&format!("[agent] ticket {ticket_id}: tool '{name}' result={result}"));
            history.push(Message::tool_result(tc.id.clone(), result.clone()));

            action_count += 1;

            if is_terminal_tool(name) {
                terminal = true;
            }
        }

        if terminal {
            logger.info(&format!("[agent] ticket {ticket_id}: terminal tool called, ending cycle"));
            break;
        }
    }

    if action_count > 0 {
        logger.info(&format!("[agent] ticket {ticket_id}: took {action_count} action(s)"));
    } else {
        logger.info(&format!("[agent] ticket {ticket_id}: no actions taken"));
    }

    Ok(())
}

async fn handle_dispatch_tool<C>(
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
    let memory_store = match MemoryStore::open(&db_path) {
        Ok(m) => m,
        Err(e) => return format!("error opening memory: {e}"),
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
        &memory_store,
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
    let tools = tool_definitions_for(&tool_names);

    logger.info(&format!(
        "[subagent:{}] starting for ticket {} (max_actions={max_actions})",
        sub_cfg.name, ticket.summary.id
    ));

    let mut action_count = 0usize;
    let mut history: Vec<Message> = vec![
        Message::system(ctx_msg.system.clone()),
        Message::user(ctx_msg.user.clone()),
    ];

    loop {
        if action_count >= max_actions {
            logger.info(&format!("[subagent:{}] hit action cap without returning", sub_cfg.name));
            return "error: subagent hit action cap without returning a result".to_string();
        }

        let req = CompletionRequest {
            model: None,
            preamble: None,
            chat_history: rig_core::one_or_many::OneOrMany::many(history.clone())
                .expect("history is non-empty"),
            documents: vec![],
            tools: tools.clone(),
            temperature: None,
            max_tokens: None,
            tool_choice: None,
            additional_params: None,
            output_schema: None,
        };

        let response = match model.completion(req).await {
            Ok(r) => r,
            Err(e) => return format!("error: LLM completion error: {e}"),
        };

        let choices: Vec<AssistantContent> = response.choice.into_iter().collect();

        let last_text = choices.iter()
            .filter_map(|c| if let AssistantContent::Text(t) = c { Some(t.text.clone()) } else { None })
            .collect::<Vec<_>>()
            .join("\n");

        let tool_calls: Vec<_> = choices.iter()
            .filter_map(|c| if let AssistantContent::ToolCall(tc) = c { Some(tc.clone()) } else { None })
            .collect();

        if let Ok(content) = rig_core::one_or_many::OneOrMany::many(choices) {
            history.push(Message::Assistant { id: None, content });
        }

        if tool_calls.is_empty() {
            logger.info(&format!("[subagent:{}] no tool calls, returning last text", sub_cfg.name));
            return last_text;
        }

        let tool_board = match build_board(config, Arc::clone(&logger)).await {
            Ok(b) => b,
            Err(e) => return format!("error building board: {e}"),
        };
        let tool_memory = match MemoryStore::open(&db_path) {
            Ok(m) => m,
            Err(e) => return format!("error opening memory: {e}"),
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
            memory_store: tool_memory,
            compaction_store: tool_compaction,
            todo_store: tool_todos,
            dry_run,
            logger: Arc::clone(&logger),
            workspace: config.workspace_base_path().map(WorkspaceStore::new),
        };

        let mut terminal = false;
        let mut result_value = String::new();

        for tc in &tool_calls {
            let name = &tc.function.name;
            let args = tc.function.arguments.to_string();

            logger.info(&format!("[subagent:{}] calling tool '{name}' args={args}", sub_cfg.name));

            let result = dispatch(name, &args, &tool_ctx).await;

            logger.debug(&format!("[subagent:{}] tool '{name}' result={result}", sub_cfg.name));

            if name == "return" {
                result_value = result.clone();
                terminal = true;
            }

            history.push(Message::tool_result(tc.id.clone(), result));
            action_count += 1;
        }

        if terminal {
            logger.info(&format!("[subagent:{}] return called, finishing", sub_cfg.name));
            return result_value;
        }
    }
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
