use std::sync::Arc;

use rig_core::completion::ToolDefinition;
use serde::{Deserialize, Serialize};

use crate::board::Board;
use crate::logging::Logger;
use crate::memory::{CompactionStore, ContextRepository, TodoStore};
use crate::workspace::WorkspaceStore;

pub struct ToolContext {
    pub ticket_id: String,
    pub agent_scope: String,
    pub board: Box<dyn Board>,
    pub compaction_store: CompactionStore,
    pub todo_store: TodoStore,
    pub context_repo: ContextRepository,
    pub dry_run: bool,
    pub logger: Arc<Logger>,
    pub workspace: Option<WorkspaceStore>,
}

fn dry_run_msg(action: &str) -> String {
    format!("[dry-run] {} would have been executed", action)
}

/// Parse tool arguments from JSON, returning the uniform `error: invalid args: ...`
/// string every dispatch function previously constructed by hand. The match-and-return
/// pattern at each call site still returns the error to short-circuit cleanly.
fn parse_args<T: serde::de::DeserializeOwned>(args: &str) -> Result<T, String> {
    serde_json::from_str(args).map_err(|e| format!("error: invalid args: {e}"))
}

macro_rules! log_action {
    ($ctx:expr, $dry_run:expr, $msg:expr) => {
        if $dry_run {
            println!("[dry-run] {}", $msg);
        } else {
            $ctx.logger.info(&format!("[agent] {}", $msg));
        }
    };
}

pub async fn dispatch(tool_name: &str, args: &str, ctx: &ToolContext) -> String {
    match tool_name {
        "comment" => dispatch_comment(args, ctx).await,
        "create_sub" => dispatch_create_sub(args, ctx).await,
        "compact" => dispatch_compact(args, ctx).await,
        "done" => dispatch_done(args, ctx).await,
        "skip" => "skip".to_string(),
        "return" => dispatch_return(args).await,
        "bash" => dispatch_bash(args, ctx).await,
        "todos" => dispatch_todos(args, ctx).await,
        "memory_list" => dispatch_memory_list(ctx).await,
        "memory_read" => dispatch_memory_read(args, ctx).await,
        "memory_write" => dispatch_memory_write(args, ctx).await,
        "memory_search" => dispatch_memory_search(args, ctx).await,
        other => format!("error: unknown tool '{other}'"),
    }
}

pub fn is_terminal_tool(tool_name: &str) -> bool {
    matches!(tool_name, "done" | "skip" | "return")
}

pub fn tool_definitions_for(names: &[&str]) -> Vec<ToolDefinition> {
    let all = all_tool_definitions();
    all.into_iter()
        .filter(|t| names.iter().any(|n| *n == t.name))
        .collect()
}

pub const MAIN_TOOLS: &[&str] = &[
    "comment",
    "dispatch",
    "skip",
    "done",
    "compact",
    "todos",
    "memory_list",
    "memory_read",
    "memory_write",
    "memory_search",
];

#[derive(Deserialize)]
struct CommentArgs {
    text: String,
}

async fn dispatch_comment(args: &str, ctx: &ToolContext) -> String {
    let parsed: CommentArgs = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };
    log_action!(
        ctx,
        ctx.dry_run,
        format!("comment on {}: {:?}", ctx.ticket_id, parsed.text)
    );
    if ctx.dry_run {
        return dry_run_msg(&format!("comment on {}", ctx.ticket_id));
    }
    match ctx.board.comment(&ctx.ticket_id, &parsed.text).await {
        Ok(()) => "comment posted".to_string(),
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize)]
struct CreateSubArgs {
    title: String,
    description: Option<String>,
    list: Option<String>,
}

async fn dispatch_create_sub(args: &str, ctx: &ToolContext) -> String {
    let parsed: CreateSubArgs = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };
    log_action!(
        ctx,
        ctx.dry_run,
        format!(
            "create sub-ticket under {}: {:?}",
            ctx.ticket_id, parsed.title
        )
    );
    if ctx.dry_run {
        return dry_run_msg(&format!(
            "create sub-ticket '{}' under {}",
            parsed.title, ctx.ticket_id
        ));
    }
    match ctx
        .board
        .create_sub(
            &ctx.ticket_id,
            &parsed.title,
            parsed.description.as_deref(),
            parsed.list.as_deref(),
        )
        .await
    {
        Ok(sub) => format!(
            "created sub-ticket: {} ({})",
            sub.summary.title, sub.summary.url
        ),
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize)]
struct MemoryReadArgs {
    path: String,
}

async fn dispatch_memory_list(ctx: &ToolContext) -> String {
    match ctx.context_repo.list() {
        Ok(entries) if entries.is_empty() => "(empty repository — no memory files yet)".to_string(),
        Ok(entries) => entries
            .iter()
            .map(|e| {
                if e.description.is_empty() {
                    e.path.clone()
                } else {
                    format!("{} — {}", e.path, e.description)
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Err(e) => format!("error: {e}"),
    }
}

async fn dispatch_memory_read(args: &str, ctx: &ToolContext) -> String {
    let parsed: MemoryReadArgs = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };
    match ctx.context_repo.read(&parsed.path) {
        Ok(content) => content,
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize)]
struct MemoryWriteArgs {
    path: String,
    content: String,
    commit_msg: String,
}

async fn dispatch_memory_write(args: &str, ctx: &ToolContext) -> String {
    let parsed: MemoryWriteArgs = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };
    log_action!(ctx, ctx.dry_run, format!("memory_write {}", parsed.path));
    if ctx.dry_run {
        return dry_run_msg(&format!("memory_write {}", parsed.path));
    }
    match ctx
        .context_repo
        .write(&parsed.path, &parsed.content, &parsed.commit_msg)
    {
        Ok(()) => format!("written: {}", parsed.path),
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize)]
struct MemorySearchArgs {
    query: String,
}

async fn dispatch_memory_search(args: &str, ctx: &ToolContext) -> String {
    let parsed: MemorySearchArgs = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };
    match ctx.context_repo.search(&parsed.query) {
        Ok(results) if results.is_empty() => "(no matches)".to_string(),
        Ok(results) => results
            .iter()
            .map(|(path, line_no, line)| format!("{}:{}: {}", path, line_no, line))
            .collect::<Vec<_>>()
            .join("\n"),
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize)]
struct CompactArgs {
    summary: String,
}

async fn dispatch_compact(args: &str, ctx: &ToolContext) -> String {
    let parsed: CompactArgs = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };
    log_action!(
        ctx,
        ctx.dry_run,
        format!("compact comments for {}", ctx.ticket_id)
    );
    if ctx.dry_run {
        return dry_run_msg(&format!("compact comments for {}", ctx.ticket_id));
    }
    let boundary = chrono::Utc::now();
    match ctx
        .compaction_store
        .set(&ctx.ticket_id, &parsed.summary, boundary, 0)
    {
        Ok(()) => "compaction stored".to_string(),
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize)]
struct DoneArgs {
    comment: Option<String>,
}

async fn dispatch_done(args: &str, ctx: &ToolContext) -> String {
    let parsed: DoneArgs = serde_json::from_str(args).unwrap_or(DoneArgs { comment: None });
    log_action!(
        ctx,
        ctx.dry_run,
        format!("return ticket {} to creator (done)", ctx.ticket_id)
    );
    if ctx.dry_run {
        return dry_run_msg(&format!("return ticket {} to creator", ctx.ticket_id));
    }
    match ctx
        .board
        .return_ticket(&ctx.ticket_id, parsed.comment.as_deref())
        .await
    {
        Ok(()) => "ticket returned to creator".to_string(),
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize, Serialize, Clone)]
struct StoredTodoItem {
    content: String,
    status: String,
    active_form: String,
}

#[derive(Deserialize)]
struct TodosItem {
    content: String,
    status: String,
    active_form: Option<String>,
}

#[derive(Deserialize)]
struct TodosArgs {
    todos: Vec<TodosItem>,
}

fn todos_scope_key(scope: &str) -> String {
    scope
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

async fn dispatch_todos(args: &str, ctx: &ToolContext) -> String {
    let parsed: TodosArgs = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };

    for item in &parsed.todos {
        match item.status.as_str() {
            "pending" | "in_progress" | "completed" => {}
            other => {
                return format!(
                    "error: invalid status {:?} for todo {:?}",
                    other, item.content
                );
            }
        }
    }

    let scope = todos_scope_key(&ctx.agent_scope);

    let new_items: Vec<StoredTodoItem> = parsed
        .todos
        .iter()
        .map(|t| StoredTodoItem {
            content: t.content.clone(),
            status: t.status.clone(),
            active_form: t.active_form.clone().unwrap_or_default(),
        })
        .collect();

    let mut pending = 0usize;
    let mut in_progress = 0usize;
    let mut completed = 0usize;

    for item in &new_items {
        match item.status.as_str() {
            "pending" => pending += 1,
            "in_progress" => in_progress += 1,
            "completed" => completed += 1,
            _ => {}
        }
    }

    let serialized = match serde_json::to_string(&new_items) {
        Ok(s) => s,
        Err(e) => return format!("error: failed to serialize todos: {e}"),
    };
    if let Err(e) = ctx.todo_store.set(&ctx.ticket_id, &scope, &serialized) {
        return format!("error: failed to save todos: {e}");
    }

    format!(
        "Todo list updated successfully.\n\nStatus: {pending} pending, {in_progress} in progress, {completed} completed\nTodos have been modified successfully. Ensure that you continue to use the todo list to track your progress. Please proceed with the current tasks if applicable."
    )
}

#[derive(Deserialize)]
pub struct ReturnArgs {
    pub result: String,
}

#[derive(Deserialize)]
pub struct DispatchArgs {
    pub subagent: String,
    pub task: String,
}

async fn dispatch_return(args: &str) -> String {
    match serde_json::from_str::<ReturnArgs>(args) {
        Ok(parsed) => parsed.result,
        Err(e) => format!("error: invalid args: {e}"),
    }
}

pub fn all_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "comment".to_string(),
            description: "Post a comment on the ticket.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "The comment text to post" }
                },
                "required": ["text"]
            }),
        },
        ToolDefinition {
            name: "create_sub".to_string(),
            description: "Create a sub-ticket linked to this ticket.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Title of the new sub-ticket" },
                    "description": { "type": "string", "description": "Optional description for the sub-ticket" },
                    "list": { "type": "string", "description": "Optional list/column name (defaults to parent's list)" }
                },
                "required": ["title"]
            }),
        },
        ToolDefinition {
            name: "compact".to_string(),
            description: "Store a compaction summary for this ticket's comments to reduce future context size.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "summary": { "type": "string", "description": "Summary of the compacted comments" }
                },
                "required": ["summary"]
            }),
        },
        ToolDefinition {
            name: "done".to_string(),
            description: "Mark work as complete and return the ticket to its creator. Use this when you have finished all work for this ticket.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "comment": { "type": "string", "description": "Optional comment to post before returning (e.g. summary of work done)" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "skip".to_string(),
            description: "Skip this ticket for this cycle without taking action. Use when the ticket is not actionable right now.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "dispatch".to_string(),
            description: "Delegate work to a specialized subagent. The subagent will run its own loop and return a result.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "subagent": { "type": "string", "description": "Name of the subagent to invoke" },
                    "task": { "type": "string", "description": "Description of what the subagent should do" }
                },
                "required": ["subagent", "task"]
            }),
        },
        ToolDefinition {
            name: "return".to_string(),
            description: "Return a result from the subagent to the main agent. This ends the subagent loop.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "result": { "type": "string", "description": "The result to return to the main agent" }
                },
                "required": ["result"]
            }),
        },
        ToolDefinition {
            name: "bash".to_string(),
            description: "Run a shell command in the ticket workspace directory. Returns structured JSON with stdout, stderr, and exit_code.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to execute (passed to sh -c)" }
                },
                "required": ["command"]
            }),
        },
        ToolDefinition {
            name: "todos".to_string(),
            description: "Manage a structured task list for multi-step work; each task has pending/in_progress/completed state. Keep exactly one task in_progress at a time. Skip for simple or single-step tasks.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "description": "The updated todo list",
                        "items": {
                            "type": "object",
                            "properties": {
                                "content": { "type": "string", "description": "What needs to be done (imperative form)" },
                                "status": { "type": "string", "enum": ["pending", "in_progress", "completed"], "description": "Task status" },
                                "active_form": { "type": "string", "description": "Present continuous form (e.g., 'Running tests')" }
                            },
                            "required": ["content", "status"]
                        }
                    }
                },
                "required": ["todos"]
            }),
        },
        ToolDefinition {
            name: "memory_list".to_string(),
            description: "List all files in the context repository with their descriptions. Use this to discover what cross-ticket knowledge exists.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "memory_read".to_string(),
            description: "Read the full content of a context repository file by its path.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to the file (e.g. 'themes/auth.md')" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "memory_write".to_string(),
            description: "Write (create or overwrite) a topic file in the context repository. Include YAML frontmatter with a `description` field. This commits the change to the git repository.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to write (e.g. 'themes/auth.md')" },
                    "content": { "type": "string", "description": "Full file content including frontmatter" },
                    "commit_msg": { "type": "string", "description": "Informative commit message describing what was learned" }
                },
                "required": ["path", "content", "commit_msg"]
            }),
        },
        ToolDefinition {
            name: "memory_search".to_string(),
            description: "Search across all context repository files for a keyword or phrase (case-insensitive). Returns matching lines with file path and line number.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query (case-insensitive literal match)" }
                },
                "required": ["query"]
            }),
        },
    ]
}

#[derive(Deserialize)]
struct BashArgs {
    command: String,
}

async fn dispatch_bash(args: &str, ctx: &ToolContext) -> String {
    let parsed: BashArgs = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return e,
    };
    let ws = match &ctx.workspace {
        Some(w) => w,
        None => return "error: workspace not configured".to_string(),
    };
    let cwd = ws.ticket_root_path(&ctx.ticket_id);
    if let Err(e) = std::fs::create_dir_all(&cwd) {
        return format!("error: could not create workspace directory: {e}");
    }
    let run = async {
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&parsed.command)
            .current_dir(&cwd)
            .output()
            .await
    };
    match tokio::time::timeout(std::time::Duration::from_secs(120), run).await {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let exit_code = output.status.code().unwrap_or(-1);
            serde_json::json!({ "stdout": stdout, "stderr": stderr, "exit_code": exit_code }).to_string()
        }
        Ok(Err(e)) => format!("error: failed to spawn process: {e}"),
        Err(_) => serde_json::json!({ "stdout": "", "stderr": "timeout: command exceeded 120s", "exit_code": -1 }).to_string(),
    }
}

// ---------------------------------------------------------------------------
// Sleep-time tool context — minimal context for reflection/defrag agents
// ---------------------------------------------------------------------------

pub struct SleepToolContext {
    pub context_repo: ContextRepository,
    pub logger: Arc<Logger>,
}

pub async fn dispatch_sleep_tool(tool_name: &str, args: &str, ctx: &SleepToolContext) -> String {
    match tool_name {
        "memory_list" => match ctx.context_repo.list() {
            Ok(entries) if entries.is_empty() => "(empty repository)".to_string(),
            Ok(entries) => entries
                .iter()
                .map(|e| {
                    if e.description.is_empty() {
                        e.path.clone()
                    } else {
                        format!("{} — {}", e.path, e.description)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Err(e) => format!("error: {e}"),
        },
        "memory_read" => {
            let parsed: MemoryReadArgs = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return e,
            };
            match ctx.context_repo.read(&parsed.path) {
                Ok(content) => content,
                Err(e) => format!("error: {e}"),
            }
        }
        "memory_write" => {
            let parsed: MemoryWriteArgs = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return e,
            };
            ctx.logger
                .info(&format!("[sleep-tool] memory_write {}", parsed.path));
            match ctx
                .context_repo
                .write(&parsed.path, &parsed.content, &parsed.commit_msg)
            {
                Ok(()) => format!("written: {}", parsed.path),
                Err(e) => format!("error: {e}"),
            }
        }
        "memory_delete" => {
            let parsed: MemoryReadArgs = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return e,
            };
            ctx.logger
                .info(&format!("[sleep-tool] memory_delete {}", parsed.path));
            match ctx.context_repo.delete(&parsed.path) {
                Ok(()) => format!("deleted: {}", parsed.path),
                Err(e) => format!("error: {e}"),
            }
        }
        other => format!("error: unknown sleep tool '{other}'"),
    }
}

pub fn defrag_tool_definitions() -> Vec<ToolDefinition> {
    let mut defs = tool_definitions_for(&["memory_list", "memory_read", "memory_write"]);
    defs.push(ToolDefinition {
        name: "memory_delete".to_string(),
        description: "Delete a file from the context repository. Blocked if the file's description terms are not covered by any other file.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Relative path of the file to delete (e.g. 'themes/old-notes.md')" }
            },
            "required": ["path"]
        }),
    });
    defs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::OrgaError;
    use async_trait::async_trait;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use tempfile::tempdir;

    struct MockBoard {
        comments: Mutex<Vec<(String, String)>>,
    }

    impl MockBoard {
        fn new() -> Self {
            Self {
                comments: Mutex::new(vec![]),
            }
        }
    }

    #[async_trait]
    impl crate::board::Board for MockBoard {
        async fn list_assigned(&self) -> Result<Vec<crate::models::TicketSummary>, OrgaError> {
            Ok(vec![])
        }
        async fn get_ticket(&self, _id: &str) -> Result<crate::models::Ticket, OrgaError> {
            Err(OrgaError::NotFound("mock".into()))
        }
        async fn comment(&self, id: &str, text: &str) -> Result<(), OrgaError> {
            self.comments
                .lock()
                .unwrap()
                .push((id.to_string(), text.to_string()));
            Ok(())
        }
        async fn assign(&self, _id: &str, _username: &str) -> Result<(), OrgaError> {
            Ok(())
        }
        async fn create_sub(
            &self,
            _parent_id: &str,
            title: &str,
            _description: Option<&str>,
            _list: Option<&str>,
        ) -> Result<crate::models::Ticket, OrgaError> {
            Err(OrgaError::NotFound(format!("mock: {title}")))
        }
        async fn list_columns(&self) -> Result<Vec<crate::models::Column>, OrgaError> {
            Ok(vec![])
        }
        async fn whoami(&self) -> Result<crate::models::Member, OrgaError> {
            Err(OrgaError::NotFound("mock".into()))
        }
        async fn return_ticket(&self, _id: &str, _comment: Option<&str>) -> Result<(), OrgaError> {
            Ok(())
        }
    }

    fn make_ctx(dry_run: bool) -> ToolContext {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("mem.db");
        let repo_path = dir.path().join("memory");
        let ctx = ToolContext {
            ticket_id: "T-1".to_string(),
            agent_scope: "main".to_string(),
            board: Box::new(MockBoard::new()),
            compaction_store: CompactionStore::open(&db_path).unwrap(),
            todo_store: crate::memory::TodoStore::open(&db_path).unwrap(),
            context_repo: crate::memory::ContextRepository::open(&repo_path, "test-agent").unwrap(),
            dry_run,
            logger: Arc::new(crate::logging::Logger::new(
                &PathBuf::from("/dev/null"),
                false,
            )),
            workspace: None,
        };
        // keep dir alive by leaking it for the test duration
        std::mem::forget(dir);
        ctx
    }

    #[tokio::test]
    async fn dispatch_comment_dry_run_returns_dry_run_msg() {
        let ctx = make_ctx(true);
        let result = dispatch("comment", r#"{"text":"hello"}"#, &ctx).await;
        assert!(result.contains("[dry-run]"));
    }

    #[tokio::test]
    async fn dispatch_comment_executes_when_not_dry_run() {
        let ctx = make_ctx(false);
        let result = dispatch("comment", r#"{"text":"hello"}"#, &ctx).await;
        assert_eq!(result, "comment posted");
    }

    #[tokio::test]
    async fn dispatch_done_dry_run() {
        let ctx = make_ctx(true);
        let result = dispatch("done", r#"{}"#, &ctx).await;
        assert!(result.contains("[dry-run]"));
    }

    #[tokio::test]
    async fn dispatch_skip_always_returns_skip() {
        let ctx = make_ctx(false);
        let result = dispatch("skip", r#"{}"#, &ctx).await;
        assert_eq!(result, "skip");
    }

    #[tokio::test]
    async fn dispatch_unknown_tool_returns_error() {
        let ctx = make_ctx(false);
        let result = dispatch("nonexistent_tool", r#"{}"#, &ctx).await;
        assert!(result.starts_with("error: unknown tool"));
    }

    #[tokio::test]
    async fn dispatch_comment_invalid_args_returns_error() {
        let ctx = make_ctx(false);
        let result = dispatch("comment", r#"{"wrong_field":true}"#, &ctx).await;
        assert!(result.starts_with("error: invalid args"));
    }

    #[tokio::test]
    async fn tool_definitions_for_returns_subset() {
        let names = ["comment", "done"];
        let defs = tool_definitions_for(&names);
        assert_eq!(defs.len(), 2);
        assert!(defs.iter().any(|d| d.name == "comment"));
        assert!(defs.iter().any(|d| d.name == "done"));
    }

    #[tokio::test]
    async fn dispatch_return_returns_result_string() {
        let ctx = make_ctx(false);
        let result = dispatch("return", r#"{"result":"analysis complete"}"#, &ctx).await;
        assert_eq!(result, "analysis complete");
    }

    #[tokio::test]
    async fn is_terminal_tool_includes_return() {
        assert!(is_terminal_tool("return"));
        assert!(is_terminal_tool("done"));
        assert!(is_terminal_tool("skip"));
        assert!(!is_terminal_tool("comment"));
    }

    fn make_ctx_with_workspace(dry_run: bool) -> (ToolContext, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("mem.db");
        let repo_path = dir.path().join("memory");
        let ws = crate::workspace::WorkspaceStore::new(dir.path().to_path_buf());
        let ctx = ToolContext {
            ticket_id: "T-1".to_string(),
            agent_scope: "main".to_string(),
            board: Box::new(MockBoard::new()),
            compaction_store: CompactionStore::open(&db_path).unwrap(),
            todo_store: crate::memory::TodoStore::open(&db_path).unwrap(),
            context_repo: crate::memory::ContextRepository::open(&repo_path, "test-agent").unwrap(),
            dry_run,
            logger: Arc::new(crate::logging::Logger::new(
                &PathBuf::from("/dev/null"),
                false,
            )),
            workspace: Some(ws),
        };
        (ctx, dir)
    }

    #[tokio::test]
    async fn dispatch_bash_returns_structured_json() {
        let (ctx, _dir) = make_ctx_with_workspace(false);
        let result = dispatch("bash", r#"{"command":"echo hello"}"#, &ctx).await;
        let v: serde_json::Value = serde_json::from_str(&result).expect("should be valid JSON");
        assert_eq!(v["stdout"].as_str().unwrap().trim(), "hello");
        assert_eq!(v["stderr"].as_str().unwrap(), "");
        assert_eq!(v["exit_code"].as_i64().unwrap(), 0);
    }

    #[tokio::test]
    async fn dispatch_bash_captures_non_zero_exit_and_stderr() {
        let (ctx, _dir) = make_ctx_with_workspace(false);
        let result = dispatch(
            "bash",
            r#"{"command":"ls /nonexistent_path_xyz 2>&1; exit 1"}"#,
            &ctx,
        )
        .await;
        let v: serde_json::Value = serde_json::from_str(&result).expect("should be valid JSON");
        assert_eq!(v["exit_code"].as_i64().unwrap(), 1);
    }

    #[tokio::test]
    async fn dispatch_bash_executes_in_workspace_dir() {
        let (ctx, dir) = make_ctx_with_workspace(false);
        let expected = dir
            .path()
            .join("T-1")
            .canonicalize()
            .unwrap_or_else(|_| dir.path().join("T-1"))
            .to_string_lossy()
            .into_owned();
        let result = dispatch("bash", r#"{"command":"pwd"}"#, &ctx).await;
        let v: serde_json::Value = serde_json::from_str(&result).expect("should be valid JSON");
        let actual = v["stdout"].as_str().unwrap().trim().to_string();
        // resolve symlinks on both sides for macOS /private/var vs /var
        let actual_canon =
            std::fs::canonicalize(&actual).unwrap_or_else(|_| std::path::PathBuf::from(&actual));
        let expected_canon = std::fs::canonicalize(&expected)
            .unwrap_or_else(|_| std::path::PathBuf::from(&expected));
        assert_eq!(actual_canon, expected_canon);
    }

    #[tokio::test]
    async fn dispatch_bash_without_workspace_returns_error() {
        let ctx = make_ctx(false);
        let result = dispatch("bash", r#"{"command":"echo hi"}"#, &ctx).await;
        assert_eq!(result, "error: workspace not configured");
    }

    #[tokio::test]
    async fn dispatch_bash_executes_in_dry_run() {
        let (ctx, _dir) = make_ctx_with_workspace(true);
        let result = dispatch("bash", r#"{"command":"echo dryrun"}"#, &ctx).await;
        let v: serde_json::Value = serde_json::from_str(&result).expect("should be valid JSON");
        assert_eq!(v["stdout"].as_str().unwrap().trim(), "dryrun");
        assert_eq!(v["exit_code"].as_i64().unwrap(), 0);
    }

    #[tokio::test]
    async fn dispatch_todos_first_call_stores_list() {
        let ctx = make_ctx(false);
        let result = dispatch("todos", r#"{"todos":[{"content":"Do A","status":"pending","active_form":"Doing A"},{"content":"Do B","status":"in_progress","active_form":"Doing B"}]}"#, &ctx).await;
        assert!(result.contains("Todo list updated successfully"));
        assert!(result.contains("1 pending, 1 in progress, 0 completed"));
    }

    #[tokio::test]
    async fn dispatch_todos_invalid_status_returns_error() {
        let ctx = make_ctx(false);
        let result = dispatch(
            "todos",
            r#"{"todos":[{"content":"Task","status":"done","active_form":""}]}"#,
            &ctx,
        )
        .await;
        assert!(result.starts_with("error:"));
        assert!(result.contains("invalid status"));
    }

    #[tokio::test]
    async fn dispatch_todos_transition_tracking() {
        let ctx = make_ctx(false);
        dispatch("todos", r#"{"todos":[{"content":"Task A","status":"in_progress","active_form":"Working"},{"content":"Task B","status":"pending","active_form":""}]}"#, &ctx).await;
        let result = dispatch("todos", r#"{"todos":[{"content":"Task A","status":"completed","active_form":"Working"},{"content":"Task B","status":"in_progress","active_form":"Doing B"}]}"#, &ctx).await;
        assert!(result.contains("0 pending, 1 in progress, 1 completed"));
    }

    #[tokio::test]
    async fn dispatch_todos_scope_key_sanitization() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("mem.db");
        let repo_path = dir.path().join("memory");
        let ctx = ToolContext {
            ticket_id: "T-1".to_string(),
            agent_scope: "my-sub agent!".to_string(),
            board: Box::new(MockBoard::new()),
            compaction_store: CompactionStore::open(&db_path).unwrap(),
            todo_store: crate::memory::TodoStore::open(&db_path).unwrap(),
            context_repo: crate::memory::ContextRepository::open(&repo_path, "test-agent").unwrap(),
            dry_run: false,
            logger: Arc::new(crate::logging::Logger::new(
                &PathBuf::from("/dev/null"),
                false,
            )),
            workspace: None,
        };
        let result = dispatch(
            "todos",
            r#"{"todos":[{"content":"Task","status":"pending","active_form":""}]}"#,
            &ctx,
        )
        .await;
        assert!(result.contains("Todo list updated successfully"));
    }

    #[tokio::test]
    async fn dispatch_sleep_memory_delete_succeeds_when_covered() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join("memory");
        let repo = crate::memory::ContextRepository::open(&repo_path, "test-agent").unwrap();
        repo.write(
            "themes/auth.md",
            "---\ndescription: Auth JWT patterns\n---\n\nAuth JWT patterns covered here.",
            "add auth",
        )
        .unwrap();
        repo.write(
            "themes/notes.md",
            "---\ndescription: Auth notes\n---\n\nNotes.",
            "add notes",
        )
        .unwrap();
        let ctx = SleepToolContext {
            context_repo: crate::memory::ContextRepository::open(&repo_path, "test-agent").unwrap(),
            logger: Arc::new(crate::logging::Logger::new(
                &std::path::PathBuf::from("/dev/null"),
                false,
            )),
        };
        let result =
            dispatch_sleep_tool("memory_delete", r#"{"path":"themes/notes.md"}"#, &ctx).await;
        assert_eq!(result, "deleted: themes/notes.md");
    }

    #[tokio::test]
    async fn dispatch_sleep_memory_delete_blocked_when_unique() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join("memory");
        let repo = crate::memory::ContextRepository::open(&repo_path, "test-agent").unwrap();
        repo.write(
            "themes/obscure.md",
            "---\ndescription: Webhook retry backoff\n---\n\nContent.",
            "add obscure",
        )
        .unwrap();
        let ctx = SleepToolContext {
            context_repo: crate::memory::ContextRepository::open(&repo_path, "test-agent").unwrap(),
            logger: Arc::new(crate::logging::Logger::new(
                &std::path::PathBuf::from("/dev/null"),
                false,
            )),
        };
        let result =
            dispatch_sleep_tool("memory_delete", r#"{"path":"themes/obscure.md"}"#, &ctx).await;
        assert!(result.starts_with("error:"));
        assert!(result.contains("cannot delete"));
    }

    #[test]
    fn memory_delete_not_in_all_tool_definitions() {
        let defs = all_tool_definitions();
        assert!(!defs.iter().any(|d| d.name == "memory_delete"));
    }

    #[test]
    fn memory_delete_in_defrag_tool_definitions() {
        let defs = defrag_tool_definitions();
        assert!(defs.iter().any(|d| d.name == "memory_delete"));
    }
}
