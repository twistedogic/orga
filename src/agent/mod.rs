pub mod agents;
pub mod config;
pub mod context;
pub mod loop_runner;
pub mod skills;
pub mod tools;

use std::sync::Arc;
use std::time::Duration;

use rig_core::client::CompletionClient;
use rig_core::completion::CompletionModel;
use rig_core::completion::message::{
    AssistantContent, Message, Text as MsgText, ToolResultContent, UserContent,
};

use crate::board::build_board;
use crate::config::{AppConfig, LlmConfig};
use crate::error::OrgaError;
use crate::logging::Logger;
use crate::memory::{CompactionStore, ContextRepository, TodoStore, format_tree_index};
use crate::metrics::{AgentMetrics, TicketOutcome, ToolOutcome, ToolScope};
use crate::workspace::WorkspaceStore;

use config::{LlmClient, build_llm_client};
use context::{SkillContext, build_context};
use skills::{SkillMeta, match_skills, scan_skills};
use tools::{ToolContext, all_tool_definitions, dispatch, is_terminal_tool, tool_definitions_for};

const SLEEP_SYSTEM_PROMPT: &str = include_str!("prompts/sleep_time.md");
const DEFRAG_SYSTEM_PROMPT: &str = include_str!("prompts/defrag.md");

fn count_actions_and_detect_done(history: &[Message]) -> (usize, bool) {
    let mut actions = 0usize;
    let mut did_done = false;
    for msg in history {
        match msg {
            Message::User { content } => {
                actions += content
                    .iter()
                    .filter(|c| matches!(c, UserContent::ToolResult(_)))
                    .count();
            }
            Message::Assistant { content, .. } => {
                did_done |= content.iter().any(|c| {
                    matches!(
                        c,
                        AssistantContent::ToolCall(tc) if tc.function.name == "done"
                    )
                });
            }
            _ => {}
        }
    }
    (actions, did_done)
}

fn extract_return_value(msg: &Message) -> Option<String> {
    let Message::User { content } = msg else {
        return None;
    };
    let UserContent::ToolResult(tr) = content.iter().next()? else {
        return None;
    };
    let ToolResultContent::Text(MsgText { text }) = tr.content.iter().next()? else {
        return None;
    };
    Some(text.clone())
}

#[cfg(test)]
mod history_helpers_tests {
    use super::*;
    use rig_core::OneOrMany;
    use rig_core::completion::message::{ToolCall, ToolFunction, ToolResult};

    fn tool_result_message(text: &str) -> Message {
        Message::User {
            content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                id: "c1".to_string(),
                call_id: None,
                content: OneOrMany::one(ToolResultContent::Text(MsgText {
                    text: text.to_string(),
                })),
            })),
        }
    }

    fn done_assistant_message() -> Message {
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::ToolCall(ToolCall {
                id: "c1".to_string(),
                call_id: None,
                function: ToolFunction {
                    name: "done".to_string(),
                    arguments: serde_json::json!({}),
                },
                signature: None,
                additional_params: None,
            })),
        }
    }

    #[test]
    fn count_increments_for_tool_result() {
        let history = vec![tool_result_message("hi")];
        let (actions, _) = count_actions_and_detect_done(&history);
        assert_eq!(actions, 1);
    }

    #[test]
    fn count_does_not_increment_for_text_user_message() {
        let history = vec![Message::user("hello".to_string())];
        let (actions, _) = count_actions_and_detect_done(&history);
        assert_eq!(actions, 0);
    }

    #[test]
    fn done_assistant_message_sets_did_done() {
        let history = vec![done_assistant_message()];
        let (_, did_done) = count_actions_and_detect_done(&history);
        assert!(did_done);
    }

    #[test]
    fn non_done_tool_call_does_not_set_did_done() {
        let history = vec![Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::ToolCall(ToolCall {
                id: "c1".to_string(),
                call_id: None,
                function: ToolFunction {
                    name: "comment".to_string(),
                    arguments: serde_json::json!({}),
                },
                signature: None,
                additional_params: None,
            })),
        }];
        let (_, did_done) = count_actions_and_detect_done(&history);
        assert!(!did_done);
    }

    #[test]
    fn extract_return_value_returns_text_from_tool_result() {
        let msg = tool_result_message("the answer");
        assert_eq!(extract_return_value(&msg), Some("the answer".to_string()));
    }

    #[test]
    fn extract_return_value_returns_none_for_text_user_message() {
        let msg = Message::user("hello".to_string());
        assert_eq!(extract_return_value(&msg), None);
    }
}

pub struct RunContext<'a> {
    pub config: &'a AppConfig,
    pub logger: Arc<Logger>,
    pub metrics: Arc<AgentMetrics>,
    pub dry_run: bool,
    pub llm_cfg: &'a LlmConfig,
}

/// Infrastructure passed to `run_subagent_loop` from the dispatch tool callback.
/// Mirrors `RunContext` but without `llm_cfg` — the subagent derives its own
/// from `config.llm_config()` because it can override model/max_actions per-subagent.
pub struct SubagentDeps<'a> {
    pub config: &'a AppConfig,
    pub logger: Arc<Logger>,
    pub metrics: Arc<AgentMetrics>,
    pub dry_run: bool,
}

pub async fn run_agent(
    once: bool,
    dry_run: bool,
    config: &AppConfig,
    logger: Arc<Logger>,
) -> Result<(), OrgaError> {
    let llm_cfg = config.llm_config()?;
    let client = build_llm_client(llm_cfg)?;
    let metrics = Arc::new(AgentMetrics::new());
    let ctx = RunContext {
        config,
        logger,
        metrics,
        dry_run,
        llm_cfg,
    };

    if once {
        run_once_for_client(&ctx, &client).await
    } else {
        run_daemon(&ctx, &client).await
    }
}

async fn run_daemon(ctx: &RunContext<'_>, client: &LlmClient) -> Result<(), OrgaError> {
    if let Some(metrics_cfg) = ctx.config.metrics_config()
        && let Err(e) = bind_metrics_server(
            metrics_cfg.listen_addr(),
            Arc::clone(&ctx.metrics),
            Arc::clone(&ctx.logger),
        )
        .await
    {
        ctx.logger.warn(&format!(
            "[metrics] could not bind {}: {e}",
            metrics_cfg.listen_addr()
        ));
    }

    let interval = Duration::from_secs(ctx.llm_cfg.poll_interval_secs());
    loop {
        if let Err(e) = run_once_for_client(ctx, client).await {
            ctx.logger.error(&format!("[agent] poll cycle error: {e}"));
        }
        tokio::time::sleep(interval).await;
    }
}

async fn run_once_for_client(ctx: &RunContext<'_>, client: &LlmClient) -> Result<(), OrgaError> {
    match client {
        LlmClient::Anthropic(c) => run_poll(ctx, c).await,
        LlmClient::OpenAi(c) => run_poll(ctx, c).await,
    }
}

async fn bind_metrics_server(
    listen_addr: &str,
    metrics: Arc<AgentMetrics>,
    logger: Arc<Logger>,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    logger.info(&format!(
        "[metrics] serving on http://{listen_addr}/metrics"
    ));
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let body = metrics.encode().unwrap_or_default();
                    drop(tokio::spawn(serve_one(stream, body)));
                }
                Err(e) => logger.error(&format!("[metrics] accept error: {e}")),
            }
        }
    });
    Ok(())
}

async fn serve_one(mut stream: tokio::net::TcpStream, body: String) -> std::io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buf = [0u8; 1024];
    let _ = stream.read(&mut buf).await?;
    let response = format!(
        "HTTP/1.0 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body,
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await
}

async fn run_poll<C>(ctx: &RunContext<'_>, client: &C) -> Result<(), OrgaError>
where
    C: CompletionClient + Sync,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    let board = build_board(ctx.config, Arc::clone(&ctx.logger)).await?;
    let tickets = board.list_assigned().await?;
    let total = tickets.len();
    let actionable: Vec<_> = tickets
        .into_iter()
        .filter(|t| !t.completed && !t.last_commenter_is_agent)
        .collect();

    ctx.logger.info(&format!(
        "[agent] {total} ticket(s) assigned, {} waiting on agent",
        actionable.len()
    ));

    if actionable.is_empty() {
        return Ok(());
    }
    ctx.logger.info(&format!(
        "[agent] processing {} ticket(s)",
        actionable.len()
    ));

    for summary in actionable {
        let ticket_id = summary.id.clone();
        let started = std::time::Instant::now();
        let result = run_ticket(ctx, client, &ticket_id).await;
        let elapsed = started.elapsed();
        let outcome = match &result {
            Ok(TicketProcessingOutcome::Success) => TicketOutcome::Success,
            Ok(TicketProcessingOutcome::Skipped) => TicketOutcome::Skipped,
            Ok(TicketProcessingOutcome::CapReached) => TicketOutcome::CapReached,
            Err(_) => TicketOutcome::Error,
        };
        ctx.metrics.record_ticket(outcome, elapsed);
        if let Err(e) = result {
            ctx.logger
                .error(&format!("[agent] error processing ticket {ticket_id}: {e}"));
        }
    }

    Ok(())
}

async fn run_ticket<C>(
    ctx: &RunContext<'_>,
    client: &C,
    ticket_id: &str,
) -> Result<TicketProcessingOutcome, OrgaError>
where
    C: CompletionClient + Sync,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    let db_path = ctx.config.memory_db_path();
    let compaction_store = CompactionStore::open(&db_path)?;
    let context_repo =
        ContextRepository::open(&ctx.config.memory_repo_path(), &ctx.config.agent.name)?;

    let board = build_board(ctx.config, Arc::clone(&ctx.logger)).await?;
    let mut ticket = board.get_ticket(ticket_id).await?;

    if let Some(rec) = compaction_store.get(ticket_id)? {
        ticket.comments.retain(|c| c.at > rec.compacted_through);
        ticket.comment_compaction = Some(crate::models::CommentCompaction {
            summary: rec.summary,
            compacted_through: rec.compacted_through,
            compacted_count: rec.compacted_count,
        });
    }

    let all_skills = ctx
        .config
        .skills_path()
        .map(|path| scan_skills(&path, &ctx.logger))
        .unwrap_or_default();

    let skill_ctx = if !all_skills.is_empty() {
        let matched = match_skills(&all_skills, &ticket.summary, &ctx.logger);
        Some(SkillContext {
            available: all_skills
                .iter()
                .map(|s| (s.name.clone(), s.description.clone()))
                .collect(),
            active: matched
                .iter()
                .map(|s| (s.name.clone(), s.body.clone()))
                .collect(),
        })
    } else {
        None
    };

    let subagent_descs: Vec<(String, String)> = ctx
        .config
        .subagents
        .iter()
        .map(|s| (s.name.clone(), s.description.clone()))
        .collect();

    let agents_md = ctx
        .config
        .agents_md_path()
        .and_then(|p| std::fs::read_to_string(p).ok());

    let ctx_msg = build_context(
        &ticket,
        &context_repo,
        ctx.llm_cfg,
        ctx.config,
        skill_ctx.as_ref(),
        &subagent_descs,
        agents_md.as_deref(),
    );

    let model = client.completion_model(&ctx.llm_cfg.model);
    let tools = if ctx.config.subagents.is_empty() {
        all_tool_definitions()
    } else {
        tool_definitions_for(tools::MAIN_TOOLS)
    };
    let max_actions = ctx.llm_cfg.max_actions_per_ticket();

    ctx.logger.info(&format!(
        "[agent] processing ticket {ticket_id} (max_actions={max_actions}, dry_run={})",
        ctx.dry_run
    ));
    if ctx.dry_run {
        println!("[dry-run] processing ticket {ticket_id}");
    }

    let tool_ctx = Arc::new(ToolContext {
        ticket_id: ticket_id.to_string(),
        agent_scope: "main".to_string(),
        board,
        compaction_store,
        todo_store: TodoStore::open(&db_path)?,
        context_repo: context_repo.clone(),
        dry_run: ctx.dry_run,
        logger: Arc::clone(&ctx.logger),
        workspace: ctx.config.workspace_base_path().map(WorkspaceStore::new),
    });

    let mut history: Vec<Message> =
        vec![Message::system(ctx_msg.system), Message::user(ctx_msg.user)];

    let dispatch_ticket = Arc::new(ticket.clone());
    let dispatch_tool_ctx = Arc::clone(&tool_ctx);
    let dispatch_logger = Arc::clone(&ctx.logger);
    let dispatch_metrics = Arc::clone(&ctx.metrics);
    let dispatch_ticket_id = ticket_id.to_string();
    let dispatch_dry_run = ctx.dry_run;
    let dispatch_config = Arc::new((*ctx.config).clone());
    let (loop_outcome, _last_text) = loop_runner::run_llm_loop(
        &model,
        &mut history,
        tools,
        max_actions,
        Arc::clone(&ctx.metrics),
        &loop_runner::LlmLoopLabels {
            model: &ctx.llm_cfg.model,
            provider: &ctx.llm_cfg.provider,
            agent: "main",
        },
        move |name, args, _choices| {
            let logger = Arc::clone(&dispatch_logger);
            let ctx = Arc::clone(&dispatch_tool_ctx);
            let ticket_id = dispatch_ticket_id.clone();
            let is_terminal = is_terminal_tool(&name);
            let name_str = name.clone();
            let dry_run_local = dispatch_dry_run;
            let cfg = Arc::clone(&dispatch_config);
            let tkt = Arc::clone(&dispatch_ticket);
            let m = Arc::clone(&dispatch_metrics);
            async move {
                logger.info(&format!(
                    "[agent] ticket {ticket_id}: calling tool '{name_str}' args={args}"
                ));
                if dry_run_local {
                    println!("[dry-run] would call tool '{name_str}' with args: {args}");
                }
                let result = if name_str == "dispatch" {
                    handle_dispatch_tool(
                        &args,
                        &tkt,
                        dry_run_local,
                        client,
                        cfg.as_ref(),
                        Arc::clone(&logger),
                        Arc::clone(&m),
                    )
                    .await
                } else {
                    dispatch(&name_str, &args, &ctx).await
                };
                let call_outcome = if result.starts_with("error:") {
                    ToolOutcome::Error
                } else {
                    ToolOutcome::Ok
                };
                m.record_tool_call(&name_str, ToolScope::Main, call_outcome);
                logger.debug(&format!(
                    "[agent] ticket {ticket_id}: tool '{name_str}' result={result}"
                ));
                if is_terminal {
                    logger.info(&format!(
                        "[agent] ticket {ticket_id}: terminal tool called, ending cycle"
                    ));
                }
                (result, is_terminal)
            }
        },
    )
    .await?;

    let processing_outcome = TicketProcessingOutcome::from_loop_outcome(loop_outcome);

    let (action_count, did_done) = count_actions_and_detect_done(&history);
    if action_count > 0 {
        ctx.logger.info(&format!(
            "[agent] ticket {ticket_id}: took {action_count} action(s)"
        ));
    } else {
        ctx.logger
            .info(&format!("[agent] ticket {ticket_id}: no actions taken"));
    }

    if did_done
        && !ctx.dry_run
        && let Ok(repo) =
            ContextRepository::open(&ctx.config.memory_repo_path(), &ctx.config.agent.name)
        && let Err(e) = run_sleep_time_agent(ctx, client, &ticket, repo).await
    {
        ctx.logger.error(&format!(
            "[sleep-time] reflection error for {ticket_id}: {e}"
        ));
    }

    Ok(processing_outcome)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketProcessingOutcome {
    Success,
    Skipped,
    CapReached,
}

impl TicketProcessingOutcome {
    pub fn from_loop_outcome(outcome: loop_runner::LoopOutcome) -> Self {
        match outcome {
            loop_runner::LoopOutcome::NoToolCalls => TicketProcessingOutcome::Skipped,
            loop_runner::LoopOutcome::CapReached => TicketProcessingOutcome::CapReached,
            loop_runner::LoopOutcome::Terminal => TicketProcessingOutcome::Success,
        }
    }
}

async fn handle_dispatch_tool<C>(
    args: &str,
    ticket: &crate::models::Ticket,
    dry_run: bool,
    client: &C,
    config: &AppConfig,
    logger: Arc<Logger>,
    metrics: Arc<AgentMetrics>,
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
        return format!(
            "[dry-run] dispatch subagent '{}' with task: {} would have been executed",
            parsed.subagent, parsed.task
        );
    }
    let sub_cfg = config.subagents.iter().find(|s| s.name == parsed.subagent);
    let sub_cfg = match sub_cfg {
        Some(s) => s,
        None => {
            return format!(
                "error: no subagent named '{}' is configured",
                parsed.subagent
            );
        }
    };
    let sub_deps = SubagentDeps {
        config,
        logger: Arc::clone(&logger),
        metrics: Arc::clone(&metrics),
        dry_run,
    };
    run_subagent_loop(client, sub_cfg, ticket, &parsed.task, &sub_deps).await
}

async fn run_subagent_loop<C>(
    client: &C,
    sub_cfg: &crate::config::SubagentConfig,
    ticket: &crate::models::Ticket,
    task: &str,
    deps: &SubagentDeps<'_>,
) -> String
where
    C: CompletionClient,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    let config = deps.config;
    let logger = &deps.logger;
    let metrics = &deps.metrics;
    let dry_run = deps.dry_run;
    let llm_cfg = match config.llm_config() {
        Ok(c) => c,
        Err(e) => return format!("error: {e}"),
    };
    let db_path = config.memory_db_path();
    let context_repo = match ContextRepository::open(&config.memory_repo_path(), &config.agent.name)
    {
        Ok(r) => r,
        Err(e) => return format!("error opening context repo: {e}"),
    };

    // Build skill context for subagent
    let all_skills = config
        .skills_path()
        .map(|path| scan_skills(&path, logger))
        .unwrap_or_default();
    let skill_ctx = if !all_skills.is_empty() {
        let active: Vec<&SkillMeta> = if sub_cfg.skills.is_empty() {
            match_skills(&all_skills, &ticket.summary, logger)
        } else {
            all_skills
                .iter()
                .filter(|s| sub_cfg.skills.contains(&s.name))
                .collect()
        };
        Some(SkillContext {
            available: vec![],
            active: active
                .iter()
                .map(|s| (s.name.clone(), s.body.clone()))
                .collect(),
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
    let max_actions = sub_cfg
        .max_actions
        .unwrap_or_else(|| llm_cfg.max_actions_per_ticket());

    let mut tool_names: Vec<String> = sub_cfg.tools.clone();
    let need: &[&str] = &[
        "return",
        "todos",
        "memory_list",
        "memory_read",
        "memory_write",
        "memory_search",
    ];
    for tool in need {
        if !tool_names.iter().any(|n| n == tool) {
            tool_names.push((*tool).to_string());
        }
    }
    let tools = tool_definitions_for(&tool_names.iter().map(String::as_str).collect::<Vec<_>>());

    logger.info(&format!(
        "[subagent:{}] starting for ticket {} (max_actions={max_actions})",
        sub_cfg.name, ticket.summary.id
    ));

    let mut history: Vec<Message> = vec![
        Message::system(ctx_msg.system.clone()),
        Message::user(ctx_msg.user.clone()),
    ];

    // Open board, compaction store, todo store, and workspace once before the loop.
    let tool_board = match build_board(config, Arc::clone(logger)).await {
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
        logger: Arc::clone(logger),
        workspace: config.workspace_base_path().map(WorkspaceStore::new),
    };
    let tool_ctx = Arc::new(tool_ctx);

    let subagent_name = sub_cfg.name.clone();
    let dispatch_logger = Arc::clone(logger);
    let dispatch_tool_ctx = Arc::clone(&tool_ctx);
    let dispatch_metrics = Arc::clone(metrics);
    let dispatch_subagent_name = subagent_name.clone();
    let sub_provider = llm_cfg.provider.clone();
    let (outcome, last_text) = loop_runner::run_llm_loop(
        &model,
        &mut history,
        tools,
        max_actions,
        Arc::clone(metrics),
        &loop_runner::LlmLoopLabels {
            model: model_name,
            provider: &sub_provider,
            agent: &subagent_name,
        },
        move |name, args, _choices| {
            let logger = Arc::clone(&dispatch_logger);
            let ctx = Arc::clone(&dispatch_tool_ctx);
            let is_return = name == "return";
            let sub_name = dispatch_subagent_name.clone();
            let m = Arc::clone(&dispatch_metrics);
            async move {
                logger.info(&format!(
                    "[subagent:{sub_name}] calling tool '{name}' args={args}"
                ));
                let result = dispatch(&name, &args, &ctx).await;
                let call_outcome = if result.starts_with("error:") {
                    ToolOutcome::Error
                } else {
                    ToolOutcome::Ok
                };
                m.record_tool_call(&name, ToolScope::Subagent, call_outcome);
                logger.debug(&format!(
                    "[subagent:{sub_name}] tool '{name}' result={result}"
                ));
                (result, is_return)
            }
        },
    )
    .await
    .unwrap_or_else(|e| (loop_runner::LoopOutcome::NoToolCalls, format!("error: {e}")));

    match outcome {
        loop_runner::LoopOutcome::Terminal => {
            let last_return_value = history.last().and_then(extract_return_value);
            logger.info(&format!(
                "[subagent:{}] return called, finishing",
                sub_cfg.name
            ));
            last_return_value.unwrap_or(last_text)
        }
        loop_runner::LoopOutcome::CapReached => {
            logger.info(&format!(
                "[subagent:{}] hit action cap without returning",
                sub_cfg.name
            ));
            "error: subagent hit action cap without returning a result".to_string()
        }
        loop_runner::LoopOutcome::NoToolCalls => {
            logger.info(&format!(
                "[subagent:{}] no tool calls, returning last text",
                sub_cfg.name
            ));
            last_text
        }
    }
}

async fn run_sleep_time_agent<C>(
    ctx: &RunContext<'_>,
    client: &C,
    ticket: &crate::models::Ticket,
    context_repo: ContextRepository,
) -> Result<(), OrgaError>
where
    C: CompletionClient + Sync,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    ctx.logger.info(&format!(
        "[sleep-time] reflecting on ticket {}",
        ticket.summary.id
    ));
    let model_name = &ctx.llm_cfg.model;
    let provider = &ctx.llm_cfg.provider;
    let memory_repo_path = ctx.config.memory_repo_path();

    let entries = context_repo.list().unwrap_or_default();
    let tree_index = if entries.is_empty() {
        "(empty)".to_string()
    } else {
        format_tree_index(&entries)
    };

    let system = SLEEP_SYSTEM_PROMPT.replace("{tree_index}", &tree_index);

    let user = format!(
        "Ticket just completed: {}\n\nDescription: {}\n\n## Comments\n{}\n\nReflect and persist any valuable cross-ticket learnings.",
        ticket.summary.title,
        ticket.summary.description,
        ticket
            .comments
            .iter()
            .map(|c| format!("@{}: {}", c.who.username, c.content))
            .collect::<Vec<_>>()
            .join("\n---\n")
    );

    let model = client.completion_model(model_name);
    let sleep_tools = tool_definitions_for(&["memory_list", "memory_read", "memory_write"]);

    let mut history: Vec<Message> = vec![Message::system(system), Message::user(user)];

    let dispatch_logger = Arc::clone(&ctx.logger);
    let dispatch_metrics = Arc::clone(&ctx.metrics);
    let (outcome, _last_text) = loop_runner::run_llm_loop(
        &model,
        &mut history,
        sleep_tools,
        10,
        Arc::clone(&ctx.metrics),
        &loop_runner::LlmLoopLabels {
            model: model_name,
            provider,
            agent: "sleep",
        },
        move |name, args, _choices| {
            let logger = Arc::clone(&dispatch_logger);
            let repo_clone = context_repo.clone();
            let m = Arc::clone(&dispatch_metrics);
            async move {
                let ctx = tools::SleepToolContext {
                    context_repo: repo_clone,
                    logger: Arc::clone(&logger),
                };
                let result = tools::dispatch_sleep_tool(&name, &args, &ctx).await;
                let call_outcome = if result.starts_with("error:") {
                    ToolOutcome::Error
                } else {
                    ToolOutcome::Ok
                };
                m.record_tool_call(&name, ToolScope::Sleep, call_outcome);
                logger.debug(&format!("[sleep-time] tool '{name}' result={result}"));
                (result, false)
            }
        },
    )
    .await?;

    ctx.logger
        .debug(&format!("[sleep-time] loop ended with {:?}", outcome));

    // Check thresholds and run defrag if needed
    let fresh_repo = ContextRepository::open(&memory_repo_path, &ctx.config.agent.name)?;
    if let Ok(stats) = fresh_repo.repo_stats()
        && (stats.file_count >= ctx.config.defrag_file_threshold()
            || stats.total_size_kb >= ctx.config.defrag_size_threshold_kb())
    {
        ctx.logger
            .info("[sleep-time] threshold exceeded, running defrag");
        if let Err(e) = run_defrag_agent(ctx, client).await {
            ctx.logger.error(&format!("[sleep-time] defrag error: {e}"));
        }
    }

    ctx.logger.info(&format!(
        "[sleep-time] reflection complete for {}",
        ticket.summary.id
    ));
    Ok(())
}

async fn run_defrag_agent<C>(ctx: &RunContext<'_>, client: &C) -> Result<(), OrgaError>
where
    C: CompletionClient + Sync,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    ctx.logger.info("[defrag] starting defragmentation pass");

    let memory_repo_path = ctx.config.memory_repo_path();
    let agent_name = &ctx.config.agent.name;
    let model_name = &ctx.llm_cfg.model;
    let provider = &ctx.llm_cfg.provider;

    let repo = ContextRepository::open(&memory_repo_path, agent_name)?;
    let entries = repo.list().unwrap_or_default();
    let tree = format_tree_index(&entries);

    let system = DEFRAG_SYSTEM_PROMPT.replace("{tree}", &tree);

    let model = client.completion_model(model_name);
    let defrag_tools = tools::defrag_tool_definitions();

    let mut history: Vec<Message> = vec![
        Message::system(system),
        Message::user("Please clean up the context repository now.".to_string()),
    ];

    let dispatch_logger = Arc::clone(&ctx.logger);
    let dispatch_metrics = Arc::clone(&ctx.metrics);
    let (outcome, _last_text) = loop_runner::run_llm_loop(
        &model,
        &mut history,
        defrag_tools,
        20,
        Arc::clone(&ctx.metrics),
        &loop_runner::LlmLoopLabels {
            model: model_name,
            provider,
            agent: "defrag",
        },
        move |name, args, _choices| {
            let logger = Arc::clone(&dispatch_logger);
            let repo_clone = repo.clone();
            let m = Arc::clone(&dispatch_metrics);
            async move {
                let ctx = tools::SleepToolContext {
                    context_repo: repo_clone,
                    logger: Arc::clone(&logger),
                };
                let result = tools::dispatch_sleep_tool(&name, &args, &ctx).await;
                let call_outcome = if result.starts_with("error:") {
                    ToolOutcome::Error
                } else {
                    ToolOutcome::Ok
                };
                m.record_tool_call(&name, ToolScope::Sleep, call_outcome);
                logger.debug(&format!("[defrag] tool '{name}' result={result}"));
                (result, false)
            }
        },
    )
    .await?;

    ctx.logger
        .debug(&format!("[defrag] loop ended with {:?}", outcome));
    ctx.logger.info("[defrag] defragmentation complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use rig_core::completion::Message;
    use rig_core::completion::message::UserContent;

    fn history_entry_for(tool_name: &str, tool_call_id: &str, result: &str) -> Message {
        if tool_name == "dispatch" {
            Message::user(result)
        } else {
            Message::tool_result(tool_call_id, result)
        }
    }

    fn is_tool_result_message(msg: &Message) -> bool {
        if let Message::User { content } = msg {
            content
                .iter()
                .any(|c| matches!(c, UserContent::ToolResult(_)))
        } else {
            false
        }
    }

    fn is_plain_text_user_message(msg: &Message) -> bool {
        if let Message::User { content } = msg {
            content.iter().any(|c| matches!(c, UserContent::Text(_)))
        } else {
            false
        }
    }

    #[test]
    fn dispatch_result_is_plain_user_message_not_tool_result() {
        let msg = history_entry_for("dispatch", "call_main_abc", "subagent finished");
        assert!(
            is_plain_text_user_message(&msg),
            "dispatch should produce a plain text user message"
        );
        assert!(
            !is_tool_result_message(&msg),
            "dispatch must NOT produce a tool_result message"
        );
    }

    #[test]
    fn comment_result_is_tool_result_message() {
        let msg = history_entry_for("comment", "call_main_xyz", "comment posted");
        assert!(
            is_tool_result_message(&msg),
            "comment should produce a tool_result message"
        );
    }

    #[test]
    fn done_result_is_tool_result_message() {
        let msg = history_entry_for("done", "call_main_done", "done");
        assert!(
            is_tool_result_message(&msg),
            "done should produce a tool_result message"
        );
    }

    #[tokio::test]
    async fn bind_metrics_server_serves_text_on_metrics() {
        use crate::logging::Logger;
        use crate::metrics::AgentMetrics;
        use std::sync::Arc;
        // Pick a high random port; if it's in use, this test is a no-op (env-dependent).
        let addr = "127.0.0.1:39191";
        let logger = Arc::new(Logger::new(std::path::Path::new("/dev/null"), false));
        let metrics = Arc::new(AgentMetrics::new());
        if let Err(e) =
            super::bind_metrics_server(addr, Arc::clone(&metrics), Arc::clone(&logger)).await
        {
            eprintln!("bind_metrics_server failed (port may be in use): {e}");
            return;
        }
        // Give the spawned server a moment to start accepting.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        // Touch a metric so it appears in encode output.
        metrics.record_llm_request("m", "p", "a");
        let resp = match reqwest::get(format!("http://{addr}/metrics")).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("get /metrics failed: {e}");
                return;
            }
        };
        assert!(resp.status().is_success(), "/metrics non-2xx");
        let body = resp.text().await.unwrap();
        assert!(
            body.contains("orga_llm_requests_total"),
            "metrics body missing counter: {body}"
        );
    }
}
