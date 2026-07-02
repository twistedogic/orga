use std::process;

use crate::logging::Logger;
use crate::models::{Column, Ticket, TicketSummary};

pub fn print_column_list(columns: &[Column]) {
    for c in columns {
        println!("{}\t{}", c.id, c.name);
    }
}

pub fn print_ticket_summary_list(tickets: &[TicketSummary]) {
    for t in tickets {
        println!("[{}] {} ({}) — {}", t.id, t.title, t.list_name, t.url);
    }
}

pub fn print_ticket_detail(t: &Ticket) {
    println!("# {}", t.summary.title);
    println!("ID:        {}", t.summary.id);
    println!("List:      {}", t.summary.list_name);
    println!("URL:       {}", t.summary.url);
    println!(
        "Completed: {}",
        if t.summary.completed { "yes" } else { "no" }
    );
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
    if !t.sub_tickets.is_empty() {
        println!("\n## Sub-tickets");
        for sub in &t.sub_tickets {
            let mark = if sub.completed { "x" } else { " " };
            println!("  [{}] {} ({}) {}", mark, sub.title, sub.id, sub.url);
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
            println!(
                "  @{} at {}:",
                c.who.username,
                c.at.format("%Y-%m-%d %H:%M")
            );
            println!("    {}", c.content);
        }
        if t.compaction_suggested {
            println!(
                "  [compaction suggested: consider running `ticket compact` to reduce context]"
            );
        }
    }
}

pub fn exit_error(msg: &str, logger: &Logger) -> ! {
    logger.error(msg);
    eprintln!("error: {msg}");
    process::exit(1);
}
