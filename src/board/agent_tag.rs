//! Shared helpers for the markdown agent tag that board backends append to
//! outbound comments so they can later recognise their own comments when
//! listing tickets.
//!
//! The format `"\n\n_[orga:{agent_name}]_"` is identical for Trello and Linear
//! and has no board-specific dependencies, so it lives here once instead of
//! being duplicated per backend.

/// Append the agent tag suffix to `text`. An empty `agent_name` is a no-op so
/// backends can stay default-configurable without producing a stray suffix.
pub fn append_agent_tag(text: &str, agent_name: &str) -> String {
    if agent_name.is_empty() {
        return text.to_string();
    }
    format!("{text}\n\n_[orga:{agent_name}]_")
}

/// If `text` ends with an agent-tag suffix, return the original content and
/// the agent name; otherwise return `(text, None)` unchanged.
pub fn parse_agent_tag(text: &str) -> (String, Option<String>) {
    if let Some(pos) = text.rfind("\n\n_[orga:") {
        let suffix = &text[pos + 2..];
        if suffix.starts_with("_[orga:") && suffix.ends_with("]_") {
            let inner = &suffix[7..suffix.len() - 2];
            if !inner.is_empty() {
                return (text[..pos].to_string(), Some(inner.to_string()));
            }
        }
    }
    (text.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_agent_tag_adds_suffix() {
        let result = append_agent_tag("hello", "agent-1");
        assert_eq!(result, "hello\n\n_[orga:agent-1]_");
    }

    #[test]
    fn append_agent_tag_empty_name_unchanged() {
        let result = append_agent_tag("hello", "");
        assert_eq!(result, "hello");
    }

    #[test]
    fn parse_agent_tag_strips_and_extracts() {
        let text = "need more context\n\n_[orga:agent-1]_";
        let (content, agent_name) = parse_agent_tag(text);
        assert_eq!(content, "need more context");
        assert_eq!(agent_name, Some("agent-1".to_string()));
    }

    #[test]
    fn parse_agent_tag_no_tag_unchanged() {
        let text = "just a normal comment";
        let (content, agent_name) = parse_agent_tag(text);
        assert_eq!(content, "just a normal comment");
        assert_eq!(agent_name, None);
    }
}
