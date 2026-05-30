use std::sync::Arc;

use rig_core::completion::ToolDefinition;
use serde::Deserialize;

use crate::board::Board;
use crate::logging::Logger;
use crate::memory::{CompactionStore, MemoryStore};
use crate::workspace::WorkspaceStore;

pub struct ToolContext {
    pub ticket_id: String,
    pub board: Box<dyn Board>,
    pub memory_store: MemoryStore,
    pub compaction_store: CompactionStore,
    pub dry_run: bool,
    pub logger: Arc<Logger>,
    pub workspace: Option<WorkspaceStore>,
}

fn dry_run_msg(action: &str) -> String {
    format!("[dry-run] {} would have been executed", action)
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
        "assign" => dispatch_assign(args, ctx).await,
        "create_sub" => dispatch_create_sub(args, ctx).await,
        "set_memory" => dispatch_set_memory(args, ctx).await,
        "compact" => dispatch_compact(args, ctx).await,
        "done" => dispatch_done(args, ctx).await,
        "skip" => "skip".to_string(),
        "return" => dispatch_return(args).await,
        "read_file" => dispatch_read_file(args, ctx).await,
        "write_file" => dispatch_write_file(args, ctx).await,
        "list_files" => dispatch_list_files(ctx).await,
        "bash" => dispatch_bash(args, ctx).await,
        other => format!("error: unknown tool '{other}'"),
    }
}

pub fn is_terminal_tool(tool_name: &str) -> bool {
    matches!(tool_name, "done" | "skip" | "return")
}

pub fn tool_definitions_for(names: &[String]) -> Vec<ToolDefinition> {
    let all = all_tool_definitions();
    all.into_iter().filter(|t| names.iter().any(|n| n == &t.name)).collect()
}

#[derive(Deserialize)]
struct CommentArgs {
    text: String,
}

async fn dispatch_comment(args: &str, ctx: &ToolContext) -> String {
    let parsed: CommentArgs = match serde_json::from_str(args) {
        Ok(a) => a,
        Err(e) => return format!("error: invalid args: {e}"),
    };
    log_action!(ctx, ctx.dry_run, format!("comment on {}: {:?}", ctx.ticket_id, parsed.text));
    if ctx.dry_run {
        return dry_run_msg(&format!("comment on {}", ctx.ticket_id));
    }
    match ctx.board.comment(&ctx.ticket_id, &parsed.text) {
        Ok(()) => "comment posted".to_string(),
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize)]
struct AssignArgs {
    username: String,
}

async fn dispatch_assign(args: &str, ctx: &ToolContext) -> String {
    let parsed: AssignArgs = match serde_json::from_str(args) {
        Ok(a) => a,
        Err(e) => return format!("error: invalid args: {e}"),
    };
    log_action!(ctx, ctx.dry_run, format!("assign {} to @{}", ctx.ticket_id, parsed.username));
    if ctx.dry_run {
        return dry_run_msg(&format!("assign {} to @{}", ctx.ticket_id, parsed.username));
    }
    match ctx.board.assign(&ctx.ticket_id, &parsed.username) {
        Ok(()) => format!("assigned @{}", parsed.username),
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
    let parsed: CreateSubArgs = match serde_json::from_str(args) {
        Ok(a) => a,
        Err(e) => return format!("error: invalid args: {e}"),
    };
    log_action!(ctx, ctx.dry_run, format!("create sub-ticket under {}: {:?}", ctx.ticket_id, parsed.title));
    if ctx.dry_run {
        return dry_run_msg(&format!("create sub-ticket '{}' under {}", parsed.title, ctx.ticket_id));
    }
    match ctx.board.create_sub(&ctx.ticket_id, &parsed.title, parsed.description.as_deref(), parsed.list.as_deref()) {
        Ok(sub) => format!("created sub-ticket: {} ({})", sub.summary.title, sub.summary.url),
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize)]
struct SetMemoryArgs {
    context: String,
}

async fn dispatch_set_memory(args: &str, ctx: &ToolContext) -> String {
    let parsed: SetMemoryArgs = match serde_json::from_str(args) {
        Ok(a) => a,
        Err(e) => return format!("error: invalid args: {e}"),
    };
    log_action!(ctx, ctx.dry_run, format!("set memory for {}", ctx.ticket_id));
    if ctx.dry_run {
        return dry_run_msg(&format!("set memory for {}", ctx.ticket_id));
    }
    match ctx.memory_store.set(&ctx.ticket_id, &parsed.context) {
        Ok(()) => "memory saved".to_string(),
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize)]
struct CompactArgs {
    summary: String,
}

async fn dispatch_compact(args: &str, ctx: &ToolContext) -> String {
    let parsed: CompactArgs = match serde_json::from_str(args) {
        Ok(a) => a,
        Err(e) => return format!("error: invalid args: {e}"),
    };
    log_action!(ctx, ctx.dry_run, format!("compact comments for {}", ctx.ticket_id));
    if ctx.dry_run {
        return dry_run_msg(&format!("compact comments for {}", ctx.ticket_id));
    }
    let boundary = chrono::Utc::now();
    match ctx.compaction_store.set(&ctx.ticket_id, &parsed.summary, boundary, 0) {
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
    match ctx.board.return_ticket(&ctx.ticket_id, parsed.comment.as_deref()) {
        Ok(()) => "ticket returned to creator".to_string(),
        Err(e) => format!("error: {e}"),
    }
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

pub fn tool_definitions() -> Vec<ToolDefinition> {
    all_tool_definitions()
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
            name: "assign".to_string(),
            description: "Assign the ticket to a teammate by their board username.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "username": { "type": "string", "description": "Board username (without @)" }
                },
                "required": ["username"]
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
            name: "set_memory".to_string(),
            description: "Save working context for this ticket (private to this agent, persists across cycles).".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "context": { "type": "string", "description": "Context text to store" }
                },
                "required": ["context"]
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
            name: "read_file".to_string(),
            description: "Read a text file from the ticket workspace. Returns file content.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path within the ticket workspace" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Write text content to a file in the ticket workspace. Creates directories as needed, overwrites if exists.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path within the ticket workspace" },
                    "content": { "type": "string", "description": "Text content to write" }
                },
                "required": ["path", "content"]
            }),
        },
        ToolDefinition {
            name: "list_files".to_string(),
            description: "List all files in the ticket workspace. Returns a newline-separated flat list of relative paths.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
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
    ]
}

#[derive(Deserialize)]
struct ReadFileArgs {
    path: String,
}

async fn dispatch_read_file(args: &str, ctx: &ToolContext) -> String {
    let parsed: ReadFileArgs = match serde_json::from_str(args) {
        Ok(a) => a,
        Err(e) => return format!("error: invalid args: {e}"),
    };
    let ws = match &ctx.workspace {
        Some(w) => w,
        None => return "error: workspace not configured".to_string(),
    };
    match ws.read(&ctx.ticket_id, &parsed.path) {
        Ok(content) => content,
        Err(crate::error::OrgaError::NotFound(_)) => "error: file not found".to_string(),
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize)]
struct WriteFileArgs {
    path: String,
    content: String,
}

async fn dispatch_write_file(args: &str, ctx: &ToolContext) -> String {
    let parsed: WriteFileArgs = match serde_json::from_str(args) {
        Ok(a) => a,
        Err(e) => return format!("error: invalid args: {e}"),
    };
    log_action!(ctx, ctx.dry_run, format!("write_file '{}' for {}", parsed.path, ctx.ticket_id));
    if ctx.dry_run {
        return dry_run_msg(&format!("write_file '{}'", parsed.path));
    }
    let ws = match &ctx.workspace {
        Some(w) => w,
        None => return "error: workspace not configured".to_string(),
    };
    match ws.write(&ctx.ticket_id, &parsed.path, &parsed.content) {
        Ok(()) => format!("wrote '{}'", parsed.path),
        Err(e) => format!("error: {e}"),
    }
}

#[derive(Deserialize)]
struct BashArgs {
    command: String,
}

async fn dispatch_bash(args: &str, ctx: &ToolContext) -> String {
    let parsed: BashArgs = match serde_json::from_str(args) {
        Ok(a) => a,
        Err(e) => return format!("error: invalid args: {e}"),
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

async fn dispatch_list_files(ctx: &ToolContext) -> String {
    let ws = match &ctx.workspace {
        Some(w) => w,
        None => return "error: workspace not configured".to_string(),
    };
    match ws.list(&ctx.ticket_id) {
        Ok(listing) => listing,
        Err(e) => format!("error: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::OrgaError;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use tempfile::tempdir;

    struct MockBoard {
        comments: RefCell<Vec<(String, String)>>,
    }

    impl MockBoard {
        fn new() -> Self {
            Self { comments: RefCell::new(vec![]) }
        }
    }

    impl crate::board::Board for MockBoard {
        fn list_assigned(&self) -> Result<Vec<crate::models::TicketSummary>, OrgaError> { Ok(vec![]) }
        fn get_ticket(&self, _id: &str) -> Result<crate::models::Ticket, OrgaError> {
            Err(OrgaError::NotFound("mock".into()))
        }
        fn comment(&self, id: &str, text: &str) -> Result<(), OrgaError> {
            self.comments.borrow_mut().push((id.to_string(), text.to_string()));
            Ok(())
        }
        fn assign(&self, _id: &str, _username: &str) -> Result<(), OrgaError> { Ok(()) }
        fn create_sub(&self, _parent_id: &str, title: &str, _description: Option<&str>, _list: Option<&str>) -> Result<crate::models::Ticket, OrgaError> {
            Err(OrgaError::NotFound(format!("mock: {title}")))
        }
        fn list_columns(&self) -> Result<Vec<crate::models::Column>, OrgaError> { Ok(vec![]) }
        fn whoami(&self) -> Result<crate::models::Member, OrgaError> {
            Err(OrgaError::NotFound("mock".into()))
        }
        fn return_ticket(&self, _id: &str, _comment: Option<&str>) -> Result<(), OrgaError> { Ok(()) }
    }

    fn make_ctx(dry_run: bool) -> ToolContext {
        let dir = tempdir().unwrap();
        let db_path = dir.keep().join("mem.db");
        ToolContext {
            ticket_id: "T-1".to_string(),
            board: Box::new(MockBoard::new()),
            memory_store: MemoryStore::open(&db_path).unwrap(),
            compaction_store: CompactionStore::open(&db_path).unwrap(),
            dry_run,
            logger: Arc::new(crate::logging::Logger::new(&PathBuf::from("/dev/null"), false)),
            workspace: None,
        }
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
    async fn dispatch_set_memory_dry_run() {
        let ctx = make_ctx(true);
        let result = dispatch("set_memory", r#"{"context":"working on it"}"#, &ctx).await;
        assert!(result.contains("[dry-run]"));
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
        let names = vec!["comment".to_string(), "done".to_string()];
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
        let ws = crate::workspace::WorkspaceStore::new(dir.path().to_path_buf());
        let ctx = ToolContext {
            ticket_id: "T-1".to_string(),
            board: Box::new(MockBoard::new()),
            memory_store: MemoryStore::open(&db_path).unwrap(),
            compaction_store: CompactionStore::open(&db_path).unwrap(),
            dry_run,
            logger: Arc::new(crate::logging::Logger::new(&PathBuf::from("/dev/null"), false)),
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
        let result = dispatch("bash", r#"{"command":"ls /nonexistent_path_xyz 2>&1; exit 1"}"#, &ctx).await;
        let v: serde_json::Value = serde_json::from_str(&result).expect("should be valid JSON");
        assert_eq!(v["exit_code"].as_i64().unwrap(), 1);
    }

    #[tokio::test]
    async fn dispatch_bash_executes_in_workspace_dir() {
        let (ctx, dir) = make_ctx_with_workspace(false);
        let expected = dir.path().join("T-1").canonicalize().unwrap_or_else(|_| dir.path().join("T-1")).to_string_lossy().into_owned();
        let result = dispatch("bash", r#"{"command":"pwd"}"#, &ctx).await;
        let v: serde_json::Value = serde_json::from_str(&result).expect("should be valid JSON");
        let actual = v["stdout"].as_str().unwrap().trim().to_string();
        // resolve symlinks on both sides for macOS /private/var vs /var
        let actual_canon = std::fs::canonicalize(&actual).unwrap_or_else(|_| std::path::PathBuf::from(&actual));
        let expected_canon = std::fs::canonicalize(&expected).unwrap_or_else(|_| std::path::PathBuf::from(&expected));
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
}
