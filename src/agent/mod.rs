pub mod config;
pub mod context;
pub mod tools;

use std::sync::Arc;
use std::time::Duration;

use rig_core::client::CompletionClient;
use rig_core::completion::{AssistantContent, CompletionModel, CompletionRequestBuilder, Message};

use crate::artifact::build_artifact_store;
use crate::board::build_board;
use crate::config::AppConfig;
use crate::error::OrgaError;
use crate::logging::Logger;
use crate::memory::{CompactionStore, MemoryStore};

use config::{LlmClient, build_llm_client};
use context::build_context;
use tools::{ToolContext, dispatch, is_terminal_tool, tool_definitions};

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
    C: CompletionClient,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    let board = build_board(config, Arc::clone(&logger))?;

    let tickets = board.list_assigned()?;
    let actionable: Vec<_> = tickets
        .into_iter()
        .filter(|t| !t.completed && !t.last_commenter_is_agent)
        .collect();

    if actionable.is_empty() {
        logger.info("[agent] no tickets waiting on agent");
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
    C: CompletionClient,
    C::CompletionModel: CompletionModel + Clone + 'static,
{
    let llm_cfg = config.llm_config()?;
    let board = build_board(config, Arc::clone(&logger))?;
    let db_path = config.memory_db_path();
    let memory_store = MemoryStore::open(&db_path)?;
    let compaction_store = CompactionStore::open(&db_path)?;
    let artifact_store_opt = build_artifact_store(config, Arc::clone(&logger)).ok();

    let mut ticket = board.get_ticket(ticket_id)?;

    if let Some(rec) = compaction_store.get(ticket_id)? {
        ticket.comments.retain(|c| c.at > rec.compacted_through);
        ticket.comment_compaction = Some(crate::models::CommentCompaction {
            summary: rec.summary,
            compacted_through: rec.compacted_through,
            compacted_count: rec.compacted_count,
        });
    }

    let ctx_msg = build_context(
        &ticket,
        &memory_store,
        artifact_store_opt.as_ref(),
        llm_cfg,
        config,
    );

    let model = client.completion_model(&llm_cfg.model);
    let tools = tool_definitions();
    let max_actions = llm_cfg.max_actions_per_ticket();

    logger.info(&format!("[agent] processing ticket {ticket_id} (max_actions={max_actions}, dry_run={dry_run})"));
    if dry_run {
        println!("[dry-run] processing ticket {ticket_id}");
    }

    let mut action_count = 0usize;
    let mut history: Vec<Message> = Vec::new();

    loop {
        if action_count >= max_actions {
            logger.info(&format!("[agent] ticket {ticket_id}: max actions cap ({max_actions}) reached"));
            break;
        }

        let prompt_msg = if action_count == 0 {
            ctx_msg.user.clone()
        } else {
            "Continue working on the ticket based on the tool results above.".to_string()
        };

        let req = CompletionRequestBuilder::new(model.clone(), prompt_msg)
            .preamble(ctx_msg.system.clone())
            .messages(history.clone())
            .tools(tools.clone());

        let response = req.send().await.map_err(|e| {
            OrgaError::BackendError(format!("LLM completion error for {ticket_id}: {e}"))
        })?;

        let choices: Vec<AssistantContent> = response.choice.into_iter().collect();

        let tool_calls: Vec<_> = choices
            .iter()
            .filter_map(|c| if let AssistantContent::ToolCall(tc) = c { Some(tc.clone()) } else { None })
            .collect();

        history.push(Message::assistant(
            choices.iter()
                .filter_map(|c| if let AssistantContent::Text(t) = c { Some(t.text.clone()) } else { None })
                .collect::<Vec<_>>()
                .join("\n")
        ));

        if tool_calls.is_empty() {
            logger.info(&format!("[agent] ticket {ticket_id}: LLM returned no tool calls, ending cycle"));
            break;
        }

        let tool_board = build_board(config, Arc::clone(&logger))?;
        let tool_memory = MemoryStore::open(&db_path)?;
        let tool_compaction = CompactionStore::open(&db_path)?;
        let tool_artifact = build_artifact_store(config, Arc::clone(&logger)).ok();

        let tool_ctx = ToolContext {
            ticket_id: ticket_id.to_string(),
            board: tool_board,
            memory_store: tool_memory,
            artifact_store: tool_artifact,
            compaction_store: tool_compaction,
            dry_run,
            logger: Arc::clone(&logger),
        };

        let mut terminal = false;
        for tc in &tool_calls {
            let name = &tc.function.name;
            let args = tc.function.arguments.to_string();

            logger.info(&format!("[agent] ticket {ticket_id}: calling tool '{name}'"));
            if dry_run && name != "get_artifact" {
                println!("[dry-run] would call tool '{name}' with args: {args}");
            }

            let result = dispatch(name, &args, &tool_ctx).await;

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

    Ok(())
}
