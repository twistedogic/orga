use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use crate::logging::Logger;
use crate::models::TicketSummary;

#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub body: String,
    pub match_always: bool,
    pub match_column: Option<String>,
    pub match_label: Option<String>,
}

pub fn scan_skills(path: &Path, logger: &Arc<Logger>) -> Vec<SkillMeta> {
    if !path.exists() {
        logger.warn(&format!("[skills] skills folder not found: {}", path.display()));
        return vec![];
    }

    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(err) => {
            logger.warn(&format!("[skills] cannot read skills folder {}: {err}", path.display()));
            return vec![];
        }
    };

    let mut skills = Vec::new();
    for entry in entries.flatten() {
        let skill_dir = entry.path();
        if !skill_dir.is_dir() {
            continue;
        }
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        let content = match std::fs::read_to_string(&skill_md) {
            Ok(c) => c,
            Err(err) => {
                logger.warn(&format!("[skills] cannot read {}: {err}", skill_md.display()));
                continue;
            }
        };
        match parse_skill(&content) {
            Some(skill) => skills.push(skill),
            None => {
                logger.warn(&format!(
                    "[skills] skipping {}: invalid or missing frontmatter",
                    skill_md.display()
                ));
            }
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

fn parse_skill(content: &str) -> Option<SkillMeta> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }
    let rest = &content[3..];
    let end = rest.find("\n---")?;
    let frontmatter = &rest[..end];
    let body = rest[end + 4..].trim_start().to_string();

    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut in_metadata = false;
    let mut match_always = false;
    let mut match_column: Option<String> = None;
    let mut match_label: Option<String> = None;

    for line in frontmatter.lines() {
        let line = line.trim();
        if line == "metadata:" {
            in_metadata = true;
            continue;
        }
        if in_metadata {
            if let Some(stripped) = line.strip_prefix("orga-match-always:") {
                let val = stripped.trim().trim_matches('"');
                match_always = val == "true";
            } else if let Some(stripped) = line.strip_prefix("orga-match-column:") {
                let val = stripped.trim().trim_matches('"');
                match_column = Some(val.to_string());
            } else if let Some(stripped) = line.strip_prefix("orga-match-label:") {
                let val = stripped.trim().trim_matches('"');
                match_label = Some(val.to_string());
            } else if !line.starts_with(' ') && !line.is_empty() {
                in_metadata = false;
            }
        }
        if !in_metadata {
            if let Some(stripped) = line.strip_prefix("name:") {
                let val = stripped.trim().trim_matches('"');
                name = Some(val.to_string());
            } else if let Some(stripped) = line.strip_prefix("description:") {
                let val = stripped.trim().trim_matches('"');
                description = Some(val.to_string());
            }
        }
    }

    let name = name.filter(|n| !n.is_empty())?;
    let description = description.unwrap_or_default();

    Some(SkillMeta {
        name,
        description,
        body,
        match_always,
        match_column,
        match_label,
    })
}

pub fn match_skills<'a>(
    skills: &'a [SkillMeta],
    ticket: &TicketSummary,
    logger: &Arc<Logger>,
) -> Vec<&'a SkillMeta> {
    let ticket_column = ticket.list_name.to_lowercase();
    let ticket_labels_lower: Vec<String> = ticket.labels.iter().map(|l| l.to_lowercase()).collect();

    let skill_label_requests: Vec<String> = ticket_labels_lower
        .iter()
        .filter_map(|l| l.strip_prefix("skill:").map(|s| s.to_string()))
        .collect();

    let mut seen: HashSet<String> = HashSet::new();
    let mut matched: Vec<&SkillMeta> = Vec::new();

    for skill in skills {
        let mut activates = false;

        if skill.match_always {
            activates = true;
        }
        if let Some(ref col) = skill.match_column
            && col.to_lowercase() == ticket_column {
                activates = true;
            }
        if let Some(ref lbl) = skill.match_label
            && ticket_labels_lower.iter().any(|l| l == &lbl.to_lowercase()) {
                activates = true;
            }
        if skill_label_requests.iter().any(|r| r == &skill.name) {
            activates = true;
        }

        if activates && !seen.contains(&skill.name) {
            seen.insert(skill.name.clone());
            matched.push(skill);
        }
    }

    for requested in &skill_label_requests {
        if !seen.contains(requested.as_str()) {
            logger.warn(&format!(
                "[skills] ticket {} requested skill '{requested}' via label but no such skill was found",
                ticket.id
            ));
        }
    }

    matched
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::logging::Logger;
    use crate::models::{Member, TicketSummary};

    fn make_logger() -> Arc<Logger> {
        Arc::new(Logger::new(std::path::Path::new("/dev/null"), false))
    }

    fn make_ticket(column: &str, labels: Vec<&str>) -> TicketSummary {
        TicketSummary {
            id: "T-1".into(),
            title: "Test".into(),
            description: "".into(),
            list_id: "l1".into(),
            list_name: column.into(),
            url: "https://example.com".into(),
            completed: false,
            creator: Some(Member { id: "u1".into(), username: "alice".into(), full_name: "Alice".into() }),
            last_commenter_is_agent: false,
            labels: labels.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn parse(content: &str) -> Option<SkillMeta> {
        parse_skill(content)
    }

    #[test]
    fn parse_valid_frontmatter() {
        let content = "---\nname: my-skill\ndescription: Does things.\n---\nBody here.";
        let skill = parse(content).unwrap();
        assert_eq!(skill.name, "my-skill");
        assert_eq!(skill.description, "Does things.");
        assert_eq!(skill.body, "Body here.");
    }

    #[test]
    fn parse_missing_name_returns_none() {
        let content = "---\ndescription: Does things.\n---\nBody.";
        assert!(parse(content).is_none());
    }

    #[test]
    fn parse_malformed_no_opening_delimiter_returns_none() {
        let content = "name: my-skill\ndescription: x\n---\nBody.";
        assert!(parse(content).is_none());
    }

    #[test]
    fn parse_match_always_metadata() {
        let content = "---\nname: global\ndescription: x\nmetadata:\n  orga-match-always: \"true\"\n---\nBody.";
        let skill = parse(content).unwrap();
        assert!(skill.match_always);
    }

    #[test]
    fn parse_match_column_metadata() {
        let content = "---\nname: s\ndescription: x\nmetadata:\n  orga-match-column: \"Review\"\n---\nBody.";
        let skill = parse(content).unwrap();
        assert_eq!(skill.match_column.as_deref(), Some("Review"));
    }

    #[test]
    fn parse_match_label_metadata() {
        let content = "---\nname: s\ndescription: x\nmetadata:\n  orga-match-label: \"security\"\n---\nBody.";
        let skill = parse(content).unwrap();
        assert_eq!(skill.match_label.as_deref(), Some("security"));
    }

    fn make_skill(name: &str, match_always: bool, match_column: Option<&str>, match_label: Option<&str>) -> SkillMeta {
        SkillMeta {
            name: name.into(),
            description: "desc".into(),
            body: "body".into(),
            match_always,
            match_column: match_column.map(|s| s.into()),
            match_label: match_label.map(|s| s.into()),
        }
    }

    #[test]
    fn match_always_activates_regardless_of_ticket() {
        let logger = make_logger();
        let skills = vec![make_skill("global", true, None, None)];
        let ticket = make_ticket("Backlog", vec![]);
        let matched = match_skills(&skills, &ticket, &logger);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "global");
    }

    #[test]
    fn match_column_case_insensitive() {
        let logger = make_logger();
        let skills = vec![make_skill("review", false, Some("review"), None)];
        let ticket = make_ticket("Review", vec![]);
        let matched = match_skills(&skills, &ticket, &logger);
        assert_eq!(matched.len(), 1);
    }

    #[test]
    fn match_label() {
        let logger = make_logger();
        let skills = vec![make_skill("sec", false, None, Some("security"))];
        let ticket = make_ticket("Backlog", vec!["security"]);
        let matched = match_skills(&skills, &ticket, &logger);
        assert_eq!(matched.len(), 1);
    }

    #[test]
    fn skill_label_activates_named_skill() {
        let logger = make_logger();
        let skills = vec![make_skill("code-review", false, None, None)];
        let ticket = make_ticket("Backlog", vec!["skill:code-review"]);
        let matched = match_skills(&skills, &ticket, &logger);
        assert_eq!(matched.len(), 1);
    }

    #[test]
    fn skill_label_missing_skill_returns_empty_no_panic() {
        let logger = make_logger();
        let skills: Vec<SkillMeta> = vec![];
        let ticket = make_ticket("Backlog", vec!["skill:nonexistent"]);
        let matched = match_skills(&skills, &ticket, &logger);
        assert!(matched.is_empty());
    }

    #[test]
    fn deduplication_when_multiple_signals_fire() {
        let logger = make_logger();
        let skills = vec![make_skill("s", true, Some("Backlog"), None)];
        let ticket = make_ticket("Backlog", vec!["skill:s"]);
        let matched = match_skills(&skills, &ticket, &logger);
        assert_eq!(matched.len(), 1);
    }

    #[test]
    fn multiple_skills_matched_concatenated() {
        let logger = make_logger();
        let skills = vec![
            make_skill("a", true, None, None),
            make_skill("b", false, Some("Backlog"), None),
        ];
        let ticket = make_ticket("Backlog", vec![]);
        let matched = match_skills(&skills, &ticket, &logger);
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn no_match_returns_empty() {
        let logger = make_logger();
        let skills = vec![make_skill("s", false, Some("Review"), None)];
        let ticket = make_ticket("Backlog", vec![]);
        let matched = match_skills(&skills, &ticket, &logger);
        assert!(matched.is_empty());
    }
}
