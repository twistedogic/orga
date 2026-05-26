use std::process;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use serde_json::json;

use orga::agent::run_agent;
use orga::agent::skills::{match_skills, scan_skills};
use orga::artifact::build_artifact_store;
use orga::board::build_board;
use orga::config::AppConfig;
use orga::error::OrgaError;
use orga::init::run_init;
use orga::logging::Logger;
use orga::memory::{CompactionStore, MemoryStore};
use orga::models::{Column, CommentCompaction, Ticket, TicketSummary};

#[derive(Parser)]
#[command(
    name = "orga",
    about = "Kanban board CLI",
    long_about = "orga lets you interact with shared kanban boards from the command line.\n\
                  Manage tickets, communicate via comments, create sub-tickets,\n\
                  manage checklists, and track working context — all through the board."
)]
struct Cli {
    #[arg(long, global = true, help = "Path to config file (default: ~/.orga/config.toml)")]
    config: Option<String>,

    #[arg(long, global = true, help = "Output as JSON instead of human-readable text")]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Interactive setup wizard to create or update config")]
    Init,
    #[command(subcommand, about = "Manage tickets on the board")]
    Ticket(TicketCommands),
    #[command(subcommand, about = "Manage checklist items on a ticket")]
    Checklist(ChecklistCommands),
    #[command(subcommand, about = "Read and write per-ticket memory")]
    Memory(MemoryCommands),
    #[command(about = "List all columns on the board")]
    Columns,
    #[command(about = "Show the configured agent's board identity")]
    Whoami,
    #[command(subcommand, about = "Manage artifacts for a ticket")]
    Artifact(ArtifactCommands),
    #[command(about = "Run the agent loop: poll tickets and act with an LLM")]
    Agent {
        #[arg(long, help = "Process the current ticket queue once and exit")]
        once: bool,
        #[arg(long, help = "Log planned actions without executing board mutations")]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum TicketCommands {
    #[command(about = "List all tickets assigned to you")]
    List {
        #[arg(long, help = "Show only completed tickets", conflicts_with = "all")]
        completed: bool,
        #[arg(long, help = "Show all tickets regardless of status", conflicts_with = "completed")]
        all: bool,
    },
    #[command(about = "Show full details of a ticket including comments and checklists")]
    Show {
        #[arg(help = "Ticket ID")]
        id: String,
    },
    #[command(about = "Post a comment on a ticket")]
    Comment {
        #[arg(help = "Ticket ID")]
        id: String,
        #[arg(help = "Comment text")]
        text: String,
    },
    #[command(about = "Assign a ticket to a teammate by their board username")]
    Assign {
        #[arg(help = "Ticket ID")]
        id: String,
        #[arg(help = "Board username (e.g. @alice or alice)")]
        username: String,
    },
    #[command(about = "Move a ticket to a different list (column) by name")]
    Move {
        #[arg(help = "Ticket ID")]
        id: String,
        #[arg(help = "Target list name (e.g. \"In Progress\")")]
        list: String,
    },
    #[command(about = "Create a sub-ticket linked to a parent ticket")]
    CreateSub {
        #[arg(help = "Parent ticket ID")]
        parent_id: String,
        #[arg(help = "Title of the new sub-ticket")]
        title: String,
    },
    #[command(about = "Return a ticket to its creator, with an optional comment")]
    Return {
        #[arg(help = "Ticket ID")]
        id: String,
        #[arg(long, help = "Comment to post before returning")]
        comment: Option<String>,
    },
    #[command(about = "Store a compaction summary for a ticket's comments")]
    Compact {
        #[arg(help = "Ticket ID")]
        id: String,
        #[arg(long, help = "Summary text written by the agent")]
        summary: String,
    },
    #[command(about = "Delete the stored compaction record for a ticket (manual reset)")]
    Decompact {
        #[arg(help = "Ticket ID")]
        id: String,
    },
}

#[derive(Subcommand)]
enum ChecklistCommands {
    #[command(about = "Add a checklist item to a ticket")]
    Add {
        #[arg(help = "Ticket ID")]
        ticket_id: String,
        #[arg(help = "Checklist item text")]
        text: String,
    },
    #[command(about = "Mark a checklist item as complete")]
    Check {
        #[arg(help = "Ticket ID")]
        ticket_id: String,
        #[arg(help = "Checklist item ID")]
        item_id: String,
    },
}

#[derive(Subcommand)]
enum MemoryCommands {
    #[command(about = "Save working context for a ticket (overwrites previous value)")]
    Set {
        #[arg(help = "Ticket ID")]
        ticket_id: String,
        #[arg(help = "Context text to store")]
        context: String,
    },
    #[command(about = "Retrieve saved working context for a ticket")]
    Get {
        #[arg(help = "Ticket ID")]
        ticket_id: String,
    },
}

#[derive(Subcommand)]
enum ArtifactCommands {
    #[command(about = "Commit a named artifact for a ticket")]
    Commit {
        #[arg(help = "Ticket ID")]
        ticket_id: String,
        #[arg(help = "Artifact name (e.g. report.md)")]
        name: String,
        #[arg(help = "Artifact content (inline text)", conflicts_with = "file")]
        content: Option<String>,
        #[arg(long, help = "Path to file whose content will be committed", conflicts_with = "content")]
        file: Option<String>,
    },
    #[command(about = "List all artifacts for a ticket")]
    List {
        #[arg(help = "Ticket ID")]
        ticket_id: String,
    },
    #[command(about = "Get an artifact by name (scoped to current agent)")]
    Get {
        #[arg(help = "Ticket ID")]
        ticket_id: String,
        #[arg(help = "Artifact name")]
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let default_logger = Logger::new(&orga::config::expand_tilde("~/.orga/orga.log"), false);

    let is_agent = matches!(cli.command, Commands::Agent { .. });

    if is_agent {
        let rt = tokio::runtime::Runtime::new().expect("failed to build tokio runtime");
        if let Err(e) = rt.block_on(run(cli)) {
            exit_error(&e.to_string(), &default_logger);
        }
    } else if let Err(e) = run_sync(cli) {
        exit_error(&e.to_string(), &default_logger);
    }
}

async fn run(cli: Cli) -> Result<(), OrgaError> {
    let config_path = AppConfig::resolve_path(cli.config.as_deref());
    let config = AppConfig::load(&config_path)?;
    let logger = Arc::new(config.logger());

    match cli.command {
        Commands::Agent { once, dry_run } => {
            run_agent(once, dry_run, &config, Arc::clone(&logger)).await
        }
        _ => unreachable!("non-agent commands handled by run_sync"),
    }
}

fn run_sync(cli: Cli) -> Result<(), OrgaError> {
    let config_path = AppConfig::resolve_path(cli.config.as_deref());

    if let Commands::Init = cli.command {
        return run_init(&config_path);
    }

    let config = AppConfig::load(&config_path)?;
    let logger = Arc::new(config.logger());

    match cli.command {
        Commands::Init => unreachable!(),
        Commands::Agent { .. } => unreachable!("agent command handled by run()"),
        Commands::Columns => {
            let board = build_board(&config, Arc::clone(&logger))?;
            let columns = board.list_columns()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&columns).unwrap());
            } else {
                print_column_list(&columns);
            }
        }
        Commands::Whoami => {
            let board = build_board(&config, Arc::clone(&logger))?;
            let member = board.whoami()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&member).unwrap());
            } else {
                println!("@{} ({})", member.username, member.full_name);
            }
        }
        Commands::Ticket(cmd) => {
            let board = build_board(&config, Arc::clone(&logger))?;
            match cmd {
                TicketCommands::List { completed, all } => {
                    let tickets = board.list_assigned()?;
                    let tickets: Vec<_> = if all {
                        tickets
                    } else if completed {
                        tickets.into_iter().filter(|t| t.completed).collect()
                    } else {
                        tickets
                            .into_iter()
                            .filter(|t| !t.completed && !t.last_commenter_is_agent)
                            .collect()
                    };
                    let empty_message = if tickets.is_empty() {
                        Some(if all {
                            "No tickets assigned to you."
                        } else if completed {
                            "No completed tickets."
                        } else {
                            "No tickets waiting on you."
                        })
                    } else {
                        None
                    };
                    if cli.json {
                        let mut obj = serde_json::json!({ "tickets": tickets });
                        if let Some(msg) = empty_message {
                            obj["message"] = serde_json::json!(msg);
                        }
                        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
                    } else {
                        if let Some(msg) = empty_message {
                            println!("{}", msg);
                        } else {
                            print_ticket_summary_list(&tickets);
                        }
                    }
                }
                TicketCommands::Show { id } => {
                    let mut ticket = board.get_ticket(&id)?;
                    let db_path = config.memory_db_path();
                    let compaction_store = CompactionStore::open(&db_path)?;
                    if let Some(rec) = compaction_store.get(&id)? {
                        ticket.comments.retain(|c| c.at > rec.compacted_through);
                        ticket.comment_compaction = Some(CommentCompaction {
                            summary: rec.summary,
                            compacted_through: rec.compacted_through,
                            compacted_count: rec.compacted_count,
                        });
                    } else if ticket.comments.len() > config.compaction_threshold() {
                        ticket.compaction_suggested = true;
                    }
                    let workflow_prompt = config.workflow_prompt(&ticket.summary.list_name);
                    let skill_hints: Vec<(String, String)> = config
                        .skills_path()
                        .map(|p| {
                            let all = scan_skills(&p, &logger);
                            match_skills(&all, &ticket.summary, &logger)
                                .into_iter()
                                .map(|s| (s.name.clone(), s.description.clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                    if cli.json {
                        let mut val = serde_json::to_value(&ticket).unwrap();
                        if let Some(prompt) = workflow_prompt {
                            val["workflow_prompt"] = serde_json::json!(prompt);
                        }
                        val["skill_hints"] = serde_json::json!(
                            skill_hints.iter().map(|(n, d)| json!({"name": n, "description": d})).collect::<Vec<_>>()
                        );
                        println!("{}", serde_json::to_string_pretty(&val).unwrap());
                    } else {
                        print_ticket_detail(&ticket);
                        if let Some(prompt) = workflow_prompt {
                            println!("\n## Workflow\n{}", prompt);
                        }
                        if !skill_hints.is_empty() {
                            println!("\n## Skills");
                            for (name, desc) in &skill_hints {
                                println!("  {name}");
                                println!("    {desc}");
                            }
                        }
                    }
                }
                TicketCommands::Comment { id, text } => {
                    if text.is_empty() {
                        return Err(OrgaError::BackendError("comment text cannot be empty".into()));
                    }
                    board.comment(&id, &text)?;
                    if cli.json {
                        println!("{}", json!({"ok": true}));
                    } else {
                        println!("comment posted on {id}");
                    }
                }
                TicketCommands::Assign { id, username } => {
                    board.assign(&id, &username)?;
                    if cli.json {
                        println!("{}", json!({"ok": true}));
                    } else {
                        println!("assigned {username} to {id}");
                    }
                }
                TicketCommands::Move { id, list } => {
                    board.move_ticket(&id, &list)?;
                    if cli.json {
                        println!("{}", json!({"ok": true}));
                    } else {
                        println!("moved {id} to '{list}'");
                    }
                }
                TicketCommands::CreateSub { parent_id, title } => {
                    let sub = board.create_sub(&parent_id, &title)?;
                    if cli.json {
                        println!(
                            "{}",
                            json!({"id": sub.summary.id, "title": sub.summary.title, "url": sub.summary.url})
                        );
                    } else {
                        println!("created sub-ticket: {} ({})", sub.summary.title, sub.summary.url);
                    }
                }
                TicketCommands::Return { id, comment } => {
                    board.return_ticket(&id, comment.as_deref())?;
                    if cli.json {
                        println!("{}", json!({"ok": true}));
                    } else {
                        println!("returned {id} to creator");
                    }
                }
                TicketCommands::Compact { id, summary } => {
                    if summary.is_empty() {
                        return Err(OrgaError::BackendError("summary cannot be empty".into()));
                    }
                    let ticket = board.get_ticket(&id)?;
                    let boundary = ticket
                        .comments
                        .last()
                        .map(|c| c.at)
                        .unwrap_or_else(chrono::Utc::now);
                    let count = ticket.comments.len();
                    let db_path = config.memory_db_path();
                    let compaction_store = CompactionStore::open(&db_path)?;
                    compaction_store.set(&id, &summary, boundary, count)?;
                    if cli.json {
                        println!("{}", json!({"ok": true}));
                    } else {
                        println!("compaction stored for {id} ({count} comments through {})", boundary.format("%Y-%m-%d %H:%M"));
                    }
                }
                TicketCommands::Decompact { id } => {
                    let db_path = config.memory_db_path();
                    let compaction_store = CompactionStore::open(&db_path)?;
                    compaction_store.delete(&id)?;
                    if cli.json {
                        println!("{}", json!({"ok": true}));
                    } else {
                        println!("compaction record deleted for {id}");
                    }
                }
            }
        }
        Commands::Checklist(cmd) => {
            let board = build_board(&config, Arc::clone(&logger))?;
            match cmd {
                ChecklistCommands::Add { ticket_id, text } => {
                    let item_id = board.add_checklist_item(&ticket_id, &text)?;
                    if cli.json {
                        println!("{}", json!({"ok": true, "item_id": item_id}));
                    } else {
                        println!("checklist item added (id: {item_id})");
                    }
                }
                ChecklistCommands::Check { ticket_id, item_id } => {
                    board.check_item(&ticket_id, &item_id)?;
                    if cli.json {
                        println!("{}", json!({"ok": true}));
                    } else {
                        println!("item {item_id} marked complete on {ticket_id}");
                    }
                }
            }
        }
        Commands::Memory(cmd) => {
            let db_path = config.memory_db_path();
            let store = MemoryStore::open(&db_path)?;
            match cmd {
                MemoryCommands::Set { ticket_id, context } => {
                    store.set(&ticket_id, &context)?;
                    if cli.json {
                        println!("{}", json!({"ok": true}));
                    } else {
                        println!("memory saved for {ticket_id}");
                    }
                }
                MemoryCommands::Get { ticket_id } => {
                    let entry = store.get(&ticket_id)?;
                    if cli.json {
                        match entry {
                            Some(e) => println!("{}", serde_json::to_string_pretty(&e).unwrap()),
                            None => println!("{}", json!({"ticket_id": ticket_id, "context": null})),
                        }
                    } else {
                        if let Some(e) = entry {
                            println!("{}", e.context);
                        }
                    }
                }
            }
        }
        Commands::Artifact(cmd) => {
            let store = build_artifact_store(&config, Arc::clone(&logger))?;
            match cmd {
                ArtifactCommands::Commit { ticket_id, name, content, file } => {
                    let bytes: Vec<u8> = match (content, file) {
                        (Some(text), None) => text.into_bytes(),
                        (None, Some(path)) => {
                            std::fs::read(&path).map_err(|e| {
                                OrgaError::BackendError(format!("cannot read file '{path}': {e}"))
                            })?
                        }
                        (None, None) => {
                            return Err(OrgaError::BackendError(
                                "either inline content or --file must be provided".into(),
                            ))
                        }
                        (Some(_), Some(_)) => unreachable!("clap conflicts_with prevents this"),
                    };
                    let meta = store.commit(&ticket_id, &name, &bytes)?;
                    if cli.json {
                        println!(
                            "{}",
                            json!({
                                "ok": true,
                                "ticket_id": meta.ticket_id,
                                "agent_name": meta.agent_name,
                                "name": meta.name,
                                "committed_at": meta.committed_at,
                            })
                        );
                    } else {
                        println!("artifact '{}' committed for {ticket_id}", meta.name);
                    }
                }
                ArtifactCommands::List { ticket_id } => {
                    let metas = store.list(&ticket_id)?;
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&metas).unwrap());
                    } else {
                        for m in &metas {
                            println!("{}/{}\t{}", m.agent_name, m.name, m.committed_at.format("%Y-%m-%d %H:%M"));
                        }
                    }
                }
                ArtifactCommands::Get { ticket_id, name } => {
                    let artifact = store.get(&ticket_id, &name)?;
                    match artifact {
                        Some(a) => {
                            if cli.json {
                                println!("{}", serde_json::to_string_pretty(&a).unwrap());
                            } else {
                                print!("{}", a.content);
                            }
                        }
                        None => {
                            return Err(OrgaError::NotFound(format!(
                                "artifact '{name}' not found for ticket {ticket_id}"
                            )))
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn print_column_list(columns: &[Column]) {
    for c in columns {
        println!("{}\t{}", c.id, c.name);
    }
}

fn print_ticket_summary_list(tickets: &[TicketSummary]) {
    for t in tickets {
        println!("[{}] {} ({}) — {}", t.id, t.title, t.list_name, t.url);
    }
}

fn print_ticket_detail(t: &Ticket) {
    println!("# {}", t.summary.title);
    println!("ID:        {}", t.summary.id);
    println!("List:      {}", t.summary.list_name);
    println!("URL:       {}", t.summary.url);
    println!("Completed: {}", if t.summary.completed { "yes" } else { "no" });
    if let Some(ref creator) = t.summary.creator {
        println!("Creator:   @{}", creator.username);
    }
    if !t.assignees.is_empty() {
        let names: Vec<&str> = t.assignees.iter().map(|m| m.username.as_str()).collect();
        println!("Assignees: {}", names.join(", "));
    }
    if !t.summary.description.is_empty() {
        println!("\n## Description\n{}", t.summary.description);
    }
    if !t.checklists.is_empty() {
        println!("\n## Checklists");
        for cl in &t.checklists {
            println!("  {}", cl.name);
            for item in &cl.items {
                let mark = if item.complete { "x" } else { " " };
                println!("    [{}] {} ({})", mark, item.text, item.id);
            }
        }
    }
    if !t.comments.is_empty() || t.comment_compaction.is_some() || t.compaction_suggested {
        println!("\n## Comments");
        if let Some(ref cc) = t.comment_compaction {
            println!(
                "  [compacted: {} comments through {}]",
                cc.compacted_count,
                cc.compacted_through.format("%Y-%m-%d %H:%M")
            );
            println!("  Summary: {}", cc.summary);
            if !t.comments.is_empty() {
                println!("  ---");
            }
        }
        for c in &t.comments {
            println!("  @{} at {}:", c.who.username, c.at.format("%Y-%m-%d %H:%M"));
            println!("    {}", c.content);
        }
        if t.compaction_suggested {
            println!("  [compaction suggested: consider running `ticket compact` to reduce context]");
        }
    }
}

fn exit_error(msg: &str, logger: &Logger) -> ! {
    logger.error(msg);
    eprintln!("error: {msg}");
    process::exit(1);
}
