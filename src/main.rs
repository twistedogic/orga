use std::process;

use clap::{Parser, Subcommand};
use serde_json::json;

use orga::board::build_board;
use orga::config::AppConfig;
use orga::error::OrgaError;
use orga::memory::MemoryStore;
use orga::models::Ticket;

#[derive(Parser)]
#[command(name = "orga", about = "Agent-native kanban board CLI")]
struct Cli {
    #[arg(long, global = true, help = "Path to config file")]
    config: Option<String>,

    #[arg(long, global = true, help = "Output as JSON")]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(subcommand)]
    Ticket(TicketCommands),
    #[command(subcommand)]
    Checklist(ChecklistCommands),
    #[command(subcommand)]
    Memory(MemoryCommands),
}

#[derive(Subcommand)]
enum TicketCommands {
    List,
    Show { id: String },
    Comment { id: String, text: String },
    Assign { id: String, username: String },
    Move { id: String, list: String },
    CreateSub { parent_id: String, title: String },
}

#[derive(Subcommand)]
enum ChecklistCommands {
    Add { ticket_id: String, text: String },
    Check { ticket_id: String, item_id: String },
}

#[derive(Subcommand)]
enum MemoryCommands {
    Set { ticket_id: String, context: String },
    Get { ticket_id: String },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        exit_error(&e.to_string());
    }
}

fn run(cli: Cli) -> Result<(), OrgaError> {
    let config_path = AppConfig::resolve_path(cli.config.as_deref());
    let config = AppConfig::load(&config_path)?;

    match cli.command {
        Commands::Ticket(cmd) => {
            let board = build_board(&config)?;
            match cmd {
                TicketCommands::List => {
                    let tickets = board.list_assigned()?;
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&tickets).unwrap());
                    } else {
                        print_ticket_list(&tickets);
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
                            json!({"id": sub.id, "title": sub.title, "url": sub.url})
                        );
                    } else {
                        println!("created sub-ticket: {} ({})", sub.title, sub.url);
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

fn print_ticket_list(tickets: &[Ticket]) {
    if tickets.is_empty() {
        return;
    }
    for t in tickets {
        println!("[{}] {} ({}) — {}", t.id, t.title, t.list_name, t.url);
    }
}

fn print_ticket_detail(t: &Ticket) {
    println!("# {}", t.title);
    println!("ID:   {}", t.id);
    println!("List: {}", t.list_name);
    println!("URL:  {}", t.url);
    if !t.assignees.is_empty() {
        let names: Vec<&str> = t.assignees.iter().map(|m| m.username.as_str()).collect();
        println!("Assignees: {}", names.join(", "));
    }
    if !t.description.is_empty() {
        println!("\n## Description\n{}", t.description);
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
