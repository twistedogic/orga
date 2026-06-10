use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::config::SubagentConfig;
use crate::logging::Logger;

#[derive(Debug, Deserialize)]
pub(crate) struct SubagentFrontmatter {
    description: Option<String>,
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
    max_actions: Option<usize>,
}

pub(crate) fn load_markdown_agents(agents_dir: &Path, logger: &Logger) -> Vec<SubagentConfig> {
    let entries = match fs::read_dir(agents_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut agents = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                logger.warn(&format!("[agents] failed to read {}: {e}", path.display()));
                continue;
            }
        };

        match parse_markdown_agent(&name, &content) {
            Ok(agent) => agents.push(agent),
            Err(e) => {
                logger.warn(&format!("[agents] skipping {}: {e}", path.display()));
            }
        }
    }

    agents
}

pub(crate) fn parse_markdown_agent(name: &str, content: &str) -> Result<SubagentConfig, String> {
    let (frontmatter_str, body) = split_frontmatter(content)?;

    let fm: SubagentFrontmatter = serde_yaml::from_str(frontmatter_str)
        .map_err(|e| format!("invalid frontmatter YAML: {e}"))?;

    let description = fm
        .description
        .filter(|d| !d.is_empty())
        .ok_or_else(|| "missing required field: description".to_string())?;

    Ok(SubagentConfig {
        name: name.to_string(),
        description,
        tools: fm.tools,
        skills: fm.skills,
        model: None,
        max_actions: fm.max_actions,
        system_prompt: if body.trim().is_empty() {
            None
        } else {
            Some(body.trim().to_string())
        },
    })
}

fn split_frontmatter(content: &str) -> Result<(&str, &str), String> {
    if !content.starts_with("---") {
        return Err("missing frontmatter: file must start with '---'".to_string());
    }
    let after_first = &content[3..];
    let rest = after_first.strip_prefix('\n').unwrap_or(after_first);
    let close = rest
        .find("\n---")
        .ok_or_else(|| "missing closing '---' in frontmatter".to_string())?;
    let yaml = &rest[..close];
    let body = &rest[close + 4..]; // skip "\n---"
    let body = body.strip_prefix('\n').unwrap_or(body);
    Ok((yaml, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn markdown_agent_all_fields_loads_correctly() {
        let content = "---\ndescription: Does research\ntools:\n  - comment\n  - done\nskills:\n  - rust\nmax_actions: 5\n---\nYou are a researcher.\n";
        let agent = parse_markdown_agent("researcher", content).unwrap();
        assert_eq!(agent.name, "researcher");
        assert_eq!(agent.description, "Does research");
        assert_eq!(agent.tools, vec!["comment", "done"]);
        assert_eq!(agent.skills, vec!["rust"]);
        assert_eq!(agent.max_actions, Some(5));
        assert_eq!(agent.system_prompt.as_deref(), Some("You are a researcher."));
    }

    #[test]
    fn markdown_agent_description_only_uses_defaults() {
        let content = "---\ndescription: Simple agent\n---\n";
        let agent = parse_markdown_agent("simple", content).unwrap();
        assert_eq!(agent.description, "Simple agent");
        assert!(agent.tools.is_empty());
        assert!(agent.skills.is_empty());
        assert!(agent.max_actions.is_none());
        assert!(agent.system_prompt.is_none());
    }

    #[test]
    fn markdown_agent_missing_description_skips() {
        let content = "---\ntools:\n  - comment\n---\nSome prompt.\n";
        let err = parse_markdown_agent("bot", content).unwrap_err();
        assert!(err.contains("description"));
    }

    #[test]
    fn markdown_agent_malformed_yaml_skips() {
        let content = "---\n: invalid: yaml: {{\n---\nSome prompt.\n";
        let err = parse_markdown_agent("bot", content).unwrap_err();
        assert!(err.contains("YAML") || err.contains("yaml") || err.contains("frontmatter"));
    }

    #[test]
    fn markdown_agent_missing_agents_dir_returns_empty() {
        let logger = Logger::new(Path::new("/dev/null"), false);
        let result = load_markdown_agents(Path::new("/nonexistent/agents"), &logger);
        assert!(result.is_empty());
    }

    // Helper unused but kept to match the spirit of the original tests using
    // a temp file as a side effect — silence dead-code warning on the import.
    #[allow(dead_code)]
    fn _write_tmp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }
}
