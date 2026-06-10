use std::fs;
use std::path::{Path, PathBuf};

use git2::{Repository, Signature};

use crate::error::OrgaError;

pub struct ContextEntry {
    pub path: String,
    pub description: String,
}

pub struct RepoStats {
    pub file_count: usize,
    pub total_size_kb: u64,
}

#[derive(Clone)]
pub struct ContextRepository {
    root: PathBuf,
    agent_name: String,
}

impl ContextRepository {
    pub fn open(root: &Path, agent_name: &str) -> Result<Self, OrgaError> {
        fs::create_dir_all(root).map_err(|e| {
            OrgaError::BackendError(format!("cannot create memory dir {}: {e}", root.display()))
        })?;

        let repo = if root.join(".git").exists() {
            Repository::open(root)
        } else {
            Repository::init(root)
        }
        .map_err(|e| OrgaError::BackendError(format!("git repo init failed: {e}")))?;

        let system_dir = root.join("system");
        if !system_dir.exists() {
            fs::create_dir_all(&system_dir).map_err(|e| {
                OrgaError::BackendError(format!("cannot create system dir: {e}"))
            })?;
            let overview = system_dir.join("overview.md");
            fs::write(
                &overview,
                "---\ndescription: Board overview and active context\n---\n\n# Overview\n\nThis is the agent context repository. Update this file with board-level project context.\n",
            )
            .map_err(|e| OrgaError::BackendError(format!("cannot write overview: {e}")))?;
            Self::commit_all(&repo, root, agent_name, "init: create context repository")?;
        }

        Ok(Self { root: root.to_path_buf(), agent_name: agent_name.to_string() })
    }

    pub fn list(&self) -> Result<Vec<ContextEntry>, OrgaError> {
        let mut entries = Vec::new();
        Self::walk_md_files(&self.root, &self.root, &mut entries)?;
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    fn walk_md_files(
        root: &Path,
        dir: &Path,
        entries: &mut Vec<ContextEntry>,
    ) -> Result<(), OrgaError> {
        let read = fs::read_dir(dir).map_err(|e| {
            OrgaError::BackendError(format!("cannot read dir {}: {e}", dir.display()))
        })?;
        let mut sub_dirs = Vec::new();
        for entry in read {
            let entry = entry.map_err(|e| OrgaError::BackendError(format!("dir entry error: {e}")))?;
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|n| n.to_str()) == Some(".git") {
                    continue;
                }
                sub_dirs.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| OrgaError::BackendError(format!("path strip error: {e}")))?
                    .to_string_lossy()
                    .to_string();
                let content = fs::read_to_string(&path).unwrap_or_default();
                let description = extract_frontmatter_description(&content);
                entries.push(ContextEntry { path: rel, description });
            }
        }
        for sub in sub_dirs {
            Self::walk_md_files(root, &sub, entries)?;
        }
        Ok(())
    }

    pub fn read(&self, rel_path: &str) -> Result<String, OrgaError> {
        let full = self.root.join(rel_path);
        if !full.exists() {
            return Err(OrgaError::NotFound(format!("memory file not found: {rel_path}")));
        }
        fs::read_to_string(&full)
            .map_err(|e| OrgaError::BackendError(format!("cannot read {rel_path}: {e}")))
    }

    pub fn write(&self, rel_path: &str, content: &str, commit_msg: &str) -> Result<(), OrgaError> {
        let full = self.root.join(rel_path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                OrgaError::BackendError(format!("cannot create parent dirs for {rel_path}: {e}"))
            })?;
        }
        fs::write(&full, content)
            .map_err(|e| OrgaError::BackendError(format!("cannot write {rel_path}: {e}")))?;

        let repo = Repository::open(&self.root)
            .map_err(|e| OrgaError::BackendError(format!("git open failed: {e}")))?;
        Self::commit_all(&repo, &self.root, &self.agent_name, commit_msg)?;
        Ok(())
    }

    pub fn search(&self, query: &str) -> Result<Vec<(String, usize, String)>, OrgaError> {
        let mut results = Vec::new();
        let entries = self.list()?;
        let query_lower = query.to_lowercase();
        for entry in entries {
            let content = match self.read(&entry.path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            for (idx, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    results.push((entry.path.clone(), idx + 1, line.to_string()));
                }
            }
        }
        Ok(results)
    }

    pub fn repo_stats(&self) -> Result<RepoStats, OrgaError> {
        let entries = self.list()?;
        let file_count = entries.len();
        let mut total_bytes = 0u64;
        for entry in &entries {
            let full = self.root.join(&entry.path);
            if let Ok(meta) = fs::metadata(full) {
                total_bytes += meta.len();
            }
        }
        Ok(RepoStats { file_count, total_size_kb: total_bytes / 1024 })
    }

    pub fn system_files(&self) -> Result<Vec<(String, String)>, OrgaError> {
        let system_dir = self.root.join("system");
        if !system_dir.exists() {
            return Ok(vec![]);
        }
        let mut result = Vec::new();
        let read = fs::read_dir(&system_dir).map_err(|e| {
            OrgaError::BackendError(format!("cannot read system dir: {e}"))
        })?;
        let mut paths: Vec<PathBuf> = read
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
            .collect();
        paths.sort();
        for path in paths {
            let rel = path
                .strip_prefix(&self.root)
                .map_err(|e| OrgaError::BackendError(format!("path error: {e}")))?
                .to_string_lossy()
                .to_string();
            let content = fs::read_to_string(&path).unwrap_or_default();
            result.push((rel, content));
        }
        Ok(result)
    }

    fn commit_all(
        repo: &Repository,
        root: &Path,
        agent_name: &str,
        message: &str,
    ) -> Result<(), OrgaError> {
        let mut index = repo
            .index()
            .map_err(|e| OrgaError::BackendError(format!("git index error: {e}")))?;
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| OrgaError::BackendError(format!("git add error: {e}")))?;
        index
            .write()
            .map_err(|e| OrgaError::BackendError(format!("git index write error: {e}")))?;
        let oid = index
            .write_tree()
            .map_err(|e| OrgaError::BackendError(format!("git write tree error: {e}")))?;
        let tree = repo
            .find_tree(oid)
            .map_err(|e| OrgaError::BackendError(format!("git find tree error: {e}")))?;
        let sig = Signature::now(agent_name, "agent@orga")
            .map_err(|e| OrgaError::BackendError(format!("git sig error: {e}")))?;
        let _ = root; // root used for context; repo already opened from it
        match repo.head() {
            Ok(head) => {
                let parent = head
                    .peel_to_commit()
                    .map_err(|e| OrgaError::BackendError(format!("git peel error: {e}")))?;
                repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
                    .map_err(|e| OrgaError::BackendError(format!("git commit error: {e}")))?;
            }
            Err(_) => {
                repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                    .map_err(|e| OrgaError::BackendError(format!("git initial commit error: {e}")))?;
            }
        }
        Ok(())
    }

    pub fn delete(&self, rel_path: &str) -> Result<(), OrgaError> {
        let full = self.root.join(rel_path);
        if !full.exists() {
            return Err(OrgaError::NotFound(format!("memory file not found: {rel_path}")));
        }

        let content = fs::read_to_string(&full)
            .map_err(|e| OrgaError::BackendError(format!("cannot read {rel_path}: {e}")))?;
        let description = extract_frontmatter_description(&content);
        let terms = extract_significant_terms(&description);

        // If there are significant terms, verify at least one appears elsewhere
        if !terms.is_empty() {
            let mut covered = false;
            'outer: for entry in self.list()? {
                if entry.path == rel_path {
                    continue;
                }
                let other_content = match self.read(&entry.path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let other_lower = other_content.to_lowercase();
                for term in &terms {
                    if other_lower.contains(term.as_str()) {
                        covered = true;
                        break 'outer;
                    }
                }
            }
            if !covered {
                return Err(OrgaError::BackendError(format!(
                    "cannot delete '{rel_path}': no other file covers its topics (description terms: {})",
                    terms.join(", ")
                )));
            }
        }

        fs::remove_file(&full)
            .map_err(|e| OrgaError::BackendError(format!("cannot delete {rel_path}: {e}")))?;

        let repo = Repository::open(&self.root)
            .map_err(|e| OrgaError::BackendError(format!("git open failed: {e}")))?;
        let mut index = repo
            .index()
            .map_err(|e| OrgaError::BackendError(format!("git index error: {e}")))?;
        index
            .remove_path(std::path::Path::new(rel_path))
            .map_err(|e| OrgaError::BackendError(format!("git index remove error: {e}")))?;
        index
            .write()
            .map_err(|e| OrgaError::BackendError(format!("git index write error: {e}")))?;
        let oid = index
            .write_tree()
            .map_err(|e| OrgaError::BackendError(format!("git write tree error: {e}")))?;
        let tree = repo
            .find_tree(oid)
            .map_err(|e| OrgaError::BackendError(format!("git find tree error: {e}")))?;
        let sig = Signature::now(&self.agent_name, "agent@orga")
            .map_err(|e| OrgaError::BackendError(format!("git sig error: {e}")))?;
        let parent = repo
            .head()
            .map_err(|e| OrgaError::BackendError(format!("git head error: {e}")))?
            .peel_to_commit()
            .map_err(|e| OrgaError::BackendError(format!("git peel error: {e}")))?;
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("delete: {rel_path}"),
            &tree,
            &[&parent],
        )
        .map_err(|e| OrgaError::BackendError(format!("git commit error: {e}")))?;

        Ok(())
    }

    pub fn analyze(&self) -> Result<DefragReport, OrgaError> {
        let entries = self.list()?;
        let mut oversized = Vec::new();
        let mut deletion_candidates = Vec::new();

        // Build term map: path -> significant terms
        let mut term_map: Vec<(String, Vec<String>)> = Vec::new();
        // Build content map: path -> lowercased content
        let mut content_map: Vec<(String, String)> = Vec::new();

        for entry in &entries {
            let content = self.read(&entry.path).unwrap_or_default();
            let line_count = content.lines().count();
            if line_count > 200 {
                oversized.push(OversizedFile { path: entry.path.clone(), line_count });
            }
            let terms = extract_significant_terms(&entry.description);
            term_map.push((entry.path.clone(), terms));
            content_map.push((entry.path.clone(), content.to_lowercase()));
        }

        // Find duplicate pairs: ≥ 2 shared description terms
        let mut duplicates = Vec::new();
        for i in 0..term_map.len() {
            for j in (i + 1)..term_map.len() {
                let shared: Vec<String> = term_map[i].1.iter()
                    .filter(|t| term_map[j].1.contains(t))
                    .cloned()
                    .collect();
                if shared.len() >= 2 {
                    duplicates.push(DuplicatePair {
                        path_a: term_map[i].0.clone(),
                        path_b: term_map[j].0.clone(),
                        shared_terms: shared,
                    });
                }
            }
        }

        // Find deletion candidates: all description terms appear in at least one other file's content
        for (idx, (path, terms)) in term_map.iter().enumerate() {
            if terms.is_empty() {
                continue;
            }
            let mut covered_by: Option<String> = None;
            'check: for (other_idx, (other_path, other_content)) in content_map.iter().enumerate() {
                if other_idx == idx {
                    continue;
                }
                if terms.iter().all(|t| other_content.contains(t.as_str())) {
                    covered_by = Some(other_path.clone());
                    break 'check;
                }
            }
            // Looser check: at least one term covered per file
            if covered_by.is_none() {
                let all_covered = terms.iter().all(|term| {
                    content_map.iter().enumerate().any(|(other_idx, (_, other_content))| {
                        other_idx != idx && other_content.contains(term.as_str())
                    })
                });
                if all_covered {
                    // Find best covering file (one that covers the most terms)
                    if let Some((_, best_path)) = content_map.iter().enumerate()
                        .filter(|(other_idx, _)| *other_idx != idx)
                        .map(|(_, (p, c))| {
                            let count = terms.iter().filter(|t| c.contains(t.as_str())).count();
                            (count, p.clone())
                        })
                        .max_by_key(|(count, _)| *count)
                    {
                        covered_by = Some(best_path);
                    }
                }
            }
            if let Some(covering) = covered_by {
                deletion_candidates.push(DeletionCandidate {
                    path: path.clone(),
                    covered_by: covering,
                });
            }
        }

        Ok(DefragReport { oversized, duplicates, deletion_candidates })
    }
}

fn extract_frontmatter_description(content: &str) -> String {
    if !content.starts_with("---") {
        return String::new();
    }
    let after = &content[3..];
    let end = match after.find("---") {
        Some(i) => i,
        None => return String::new(),
    };
    let frontmatter = &after[..end];
    for line in frontmatter.lines() {
        if let Some(rest) = line.strip_prefix("description:") {
            return rest.trim().trim_matches('"').trim_matches('\'').to_string();
        }
    }
    String::new()
}

pub fn extract_significant_terms(description: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &["the", "and", "for", "not", "are", "was", "but", "its"];
    let mut terms: Vec<String> = description
        .split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() >= 3 && !STOPWORDS.contains(&w.as_str()))
        .collect();
    terms.sort();
    terms.dedup();
    terms
}

// ---------------------------------------------------------------------------
// DefragReport — analysis of repository health
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct OversizedFile {
    pub path: String,
    pub line_count: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct DuplicatePair {
    pub path_a: String,
    pub path_b: String,
    pub shared_terms: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct DeletionCandidate {
    pub path: String,
    pub covered_by: String,
}

#[derive(Debug, serde::Serialize)]
pub struct DefragReport {
    pub oversized: Vec<OversizedFile>,
    pub duplicates: Vec<DuplicatePair>,
    pub deletion_candidates: Vec<DeletionCandidate>,
}

impl DefragReport {
    pub fn is_empty(&self) -> bool {
        self.oversized.is_empty() && self.duplicates.is_empty() && self.deletion_candidates.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn open_temp_repo() -> (ContextRepository, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let repo = ContextRepository::open(&dir.path().join("memory"), "test-agent").unwrap();
        (repo, dir)
    }

    #[test]
    fn repo_list_returns_initial_overview() {
        let (repo, _dir) = open_temp_repo();
        let entries = repo.list().unwrap();
        assert!(entries.iter().any(|e| e.path == "system/overview.md"));
    }

    #[test]
    fn repo_list_extracts_frontmatter_description() {
        let (repo, _dir) = open_temp_repo();
        let entries = repo.list().unwrap();
        let overview = entries.iter().find(|e| e.path == "system/overview.md").unwrap();
        assert_eq!(overview.description, "Board overview and active context");
    }

    #[test]
    fn repo_read_returns_content() {
        let (repo, _dir) = open_temp_repo();
        let content = repo.read("system/overview.md").unwrap();
        assert!(content.contains("Overview"));
    }

    #[test]
    fn repo_read_missing_returns_error() {
        let (repo, _dir) = open_temp_repo();
        let result = repo.read("nonexistent/file.md");
        assert!(result.is_err());
    }

    #[test]
    fn repo_write_creates_file_and_commits() {
        let (repo, dir) = open_temp_repo();
        repo.write(
            "themes/auth.md",
            "---\ndescription: Auth patterns\n---\n\nAuth notes here.",
            "add auth theme",
        ).unwrap();
        let content = repo.read("themes/auth.md").unwrap();
        assert!(content.contains("Auth notes here."));
        // verify git commit exists
        let git_repo = git2::Repository::open(dir.path().join("memory")).unwrap();
        let head = git_repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        assert_eq!(commit.message().unwrap(), "add auth theme");
    }

    #[test]
    fn repo_write_auto_creates_parent_dirs() {
        let (repo, _dir) = open_temp_repo();
        repo.write("deeply/nested/topic.md", "content", "add nested").unwrap();
        let content = repo.read("deeply/nested/topic.md").unwrap();
        assert_eq!(content, "content");
    }

    #[test]
    fn repo_search_finds_matches_case_insensitive() {
        let (repo, _dir) = open_temp_repo();
        repo.write("themes/auth.md", "JWT tokens are used everywhere.", "add auth").unwrap();
        let results = repo.search("jwt").unwrap();
        assert!(results.iter().any(|(path, _, line)| path == "themes/auth.md" && line.contains("JWT")));
    }

    #[test]
    fn repo_search_returns_empty_for_no_match() {
        let (repo, _dir) = open_temp_repo();
        let results = repo.search("zzz_no_match_zzz").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn repo_list_missing_frontmatter_returns_empty_description() {
        let (repo, _dir) = open_temp_repo();
        repo.write("plain.md", "no frontmatter here", "add plain").unwrap();
        let entries = repo.list().unwrap();
        let plain = entries.iter().find(|e| e.path == "plain.md").unwrap();
        assert_eq!(plain.description, "");
    }

    #[test]
    fn terms_removes_stopwords() {
        let terms = extract_significant_terms("the quick and the fox");
        assert!(!terms.contains(&"the".to_string()));
        assert!(!terms.contains(&"and".to_string()));
        assert!(terms.contains(&"quick".to_string()));
        assert!(terms.contains(&"fox".to_string()));
    }

    #[test]
    fn terms_removes_short_words() {
        let terms = extract_significant_terms("do it ok go");
        assert!(terms.is_empty());
    }

    #[test]
    fn terms_strips_punctuation() {
        let terms = extract_significant_terms("auth, JWT! patterns.");
        assert!(terms.contains(&"auth".to_string()));
        assert!(terms.contains(&"jwt".to_string()));
        assert!(terms.contains(&"patterns".to_string()));
    }

    #[test]
    fn terms_lowercases_and_deduplicates() {
        let terms = extract_significant_terms("Auth auth AUTH");
        assert_eq!(terms, vec!["auth".to_string()]);
    }

    #[test]
    fn terms_empty_string_returns_empty() {
        assert!(extract_significant_terms("").is_empty());
    }

    #[test]
    fn delete_allowed_when_terms_covered_elsewhere() {
        let (repo, _dir) = open_temp_repo();
        repo.write(
            "themes/auth.md",
            "---\ndescription: Auth patterns JWT tokens\n---\n\nAuth content.",
            "add auth",
        ).unwrap();
        repo.write(
            "themes/auth-notes.md",
            "---\ndescription: Auth investigation notes\n---\n\nMore auth notes.",
            "add auth notes",
        ).unwrap();
        // "auth" appears in themes/auth.md — delete allowed
        repo.delete("themes/auth-notes.md").unwrap();
        assert!(repo.read("themes/auth-notes.md").is_err());
    }

    #[test]
    fn delete_blocked_when_terms_unique() {
        let (repo, _dir) = open_temp_repo();
        repo.write(
            "themes/obscure.md",
            "---\ndescription: Webhook retry backoff\n---\n\nContent.",
            "add obscure",
        ).unwrap();
        let result = repo.delete("themes/obscure.md");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("cannot delete"));
    }

    #[test]
    fn delete_allowed_when_no_frontmatter() {
        let (repo, _dir) = open_temp_repo();
        repo.write("plain.md", "no frontmatter here", "add plain").unwrap();
        repo.delete("plain.md").unwrap();
        assert!(repo.read("plain.md").is_err());
    }

    #[test]
    fn delete_allowed_when_empty_description() {
        let (repo, _dir) = open_temp_repo();
        repo.write(
            "empty-desc.md",
            "---\ndescription: \n---\n\nContent.",
            "add empty",
        ).unwrap();
        repo.delete("empty-desc.md").unwrap();
        assert!(repo.read("empty-desc.md").is_err());
    }

    #[test]
    fn delete_nonexistent_returns_error() {
        let (repo, _dir) = open_temp_repo();
        let result = repo.delete("nonexistent.md");
        assert!(result.is_err());
    }

    #[test]
    fn delete_produces_git_commit() {
        let (repo, dir) = open_temp_repo();
        repo.write(
            "themes/auth.md",
            "---\ndescription: Auth patterns JWT tokens\n---\n\nAuth.",
            "add auth",
        ).unwrap();
        repo.write(
            "themes/auth-notes.md",
            "---\ndescription: Auth notes\n---\n\nNotes.",
            "add notes",
        ).unwrap();
        repo.delete("themes/auth-notes.md").unwrap();
        let git_repo = git2::Repository::open(dir.path().join("memory")).unwrap();
        let msg = git_repo.head().unwrap().peel_to_commit().unwrap().message().unwrap().to_string();
        assert_eq!(msg, "delete: themes/auth-notes.md");
    }

    #[test]
    fn analyze_detects_oversized_file() {
        let (repo, _dir) = open_temp_repo();
        let big_content = format!("---\ndescription: Big file\n---\n\n{}", "line\n".repeat(210));
        repo.write("big.md", &big_content, "add big").unwrap();
        let report = repo.analyze().unwrap();
        assert!(report.oversized.iter().any(|f| f.path == "big.md" && f.line_count > 200));
    }

    #[test]
    fn analyze_detects_duplicate_pair() {
        let (repo, _dir) = open_temp_repo();
        repo.write(
            "themes/auth.md",
            "---\ndescription: Auth JWT tokens patterns\n---\n\nContent.",
            "add auth",
        ).unwrap();
        repo.write(
            "themes/auth2.md",
            "---\ndescription: Auth JWT tokens investigation\n---\n\nContent.",
            "add auth2",
        ).unwrap();
        let report = repo.analyze().unwrap();
        assert!(report.duplicates.iter().any(|d|
            (d.path_a == "themes/auth.md" && d.path_b == "themes/auth2.md") ||
            (d.path_a == "themes/auth2.md" && d.path_b == "themes/auth.md")
        ));
    }

    #[test]
    fn analyze_detects_deletion_candidate() {
        let (repo, _dir) = open_temp_repo();
        repo.write(
            "themes/auth.md",
            "---\ndescription: Auth JWT tokens patterns\n---\n\nAuth JWT tokens patterns and auth notes covered here.",
            "add auth",
        ).unwrap();
        repo.write(
            "themes/notes.md",
            "---\ndescription: Auth notes\n---\n\nJust notes.",
            "add notes",
        ).unwrap();
        let report = repo.analyze().unwrap();
        // "auth" and "notes" from notes.md description both appear in auth.md's content
        assert!(report.deletion_candidates.iter().any(|c| c.path == "themes/notes.md"));
    }

    #[test]
    fn analyze_empty_repo_report_is_empty() {
        let (repo, _dir) = open_temp_repo();
        let report = repo.analyze().unwrap();
        // Only system/overview.md exists, which is small and unique
        assert!(report.oversized.is_empty());
        assert!(report.duplicates.is_empty());
    }
}
