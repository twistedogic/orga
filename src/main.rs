use std::process;

use clap::{Parser, Subcommand};
use serde_json::json;

use orga::board::build_board;
use orga::config::AppConfig;
use orga::error::OrgaError;
use orga::init::run_init;
use orga::memory::MemoryStore;
use orga::models::{Column, Ticket, TicketSummary};

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

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        exit_error(&e.to_string());
    }
}

fn run(cli: Cli) -> Result<(), OrgaError> {
    let config_path = AppConfig::resolve_path(cli.config.as_deref());

    if let Commands::Init = cli.command {
        return run_init(&config_path);
    }

    let config = AppConfig::load(&config_path)?;

    match cli.command {
        Commands::Init => unreachable!(),
        Commands::Columns => {
            let board = build_board(&config)?;
            let columns = board.list_columns()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&columns).unwrap());
            } else {
                print_column_list(&columns);
            }
        }
        Commands::Whoami => {
            let board = build_board(&config)?;
            let member = board.whoami()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&member).unwrap());
            } else {
                println!("@{} ({})", member.username, member.full_name);
            }
        }
        Commands::Ticket(cmd) => {
            let board = build_board(&config)?;
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
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&tickets).unwrap());
                    } else {
                        print_ticket_summary_list(&tickets);
                    }
                }
                TicketCommands::Show { id } => {
                    let ticket = board.get_ticket(&id)?;
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&ticket).unwrap());
                    } else {
                        print_ticket_detail(&ticket);
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
            }
        }
        Commands::Checklist(cmd) => {
            let board = build_board(&config)?;
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
    }

    Ok(())
}

fn print_column_list(columns: &[Column]) {
    for c in columns {
        println!("{}\t{}", c.id, c.name);
    }
}

fn print_ticket_summary_list(tickets: &[TicketSummary]) {
    if tickets.is_empty() {
        return;
    }
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
    if !t.comments.is_empty() {
        println!("\n## Comments");
        for c in &t.comments {
            println!("  @{} at {}:", c.who.username, c.at.format("%Y-%m-%d %H:%M"));
            println!("    {}", c.content);
        }
    }
}

fn exit_error(msg: &str) -> ! {
    eprintln!("error: {msg}");
    process::exit(1);
}
