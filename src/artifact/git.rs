use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chrono::Utc;
use git2::{Cred, IndexAddOption, RemoteCallbacks, Repository, ResetType, Signature};

const RETRY_DELAYS_MS: [u64; 3] = [100, 200, 400];

use crate::artifact::ArtifactStore;
use crate::config::expand_tilde;
use crate::error::OrgaError;
use crate::logging::Logger;
use crate::models::{Artifact, ArtifactMeta};

#[derive(Clone, Default)]
pub struct GitAuth {
    pub ssh_key: Option<PathBuf>,
    pub ssh_passphrase: Option<String>,
    pub http_username: Option<String>,
    pub http_password: Option<String>,
}

pub struct GitArtifactStore {
    repo_path: PathBuf,
    agent_name: String,
    remote: Option<String>,
    branch: String,
    auth: GitAuth,
    logger: Arc<Logger>,
}

impl GitArtifactStore {
    pub fn new(
        path: String,
        agent_name: String,
        remote: Option<String>,
        branch: String,
        auth: GitAuth,
        logger: Arc<Logger>,
    ) -> Self {
        Self {
            repo_path: expand_tilde(&path),
            agent_name,
            remote,
            branch,
            auth,
            logger,
        }
    }

    fn open_repo(&self) -> Result<Repository, OrgaError> {
        Repository::open(&self.repo_path).map_err(|e| {
            OrgaError::ConfigError(format!(
                "cannot open artifact repo at {}: {}",
                self.repo_path.display(),
                e
            ))
        })
    }

    fn artifact_path(&self, ticket_id: &str, agent_name: &str, name: &str) -> PathBuf {
        self.repo_path
            .join("artifacts")
            .join(ticket_id)
            .join(agent_name)
            .join(name)
    }
}

impl ArtifactStore for GitArtifactStore {
    fn commit(&self, ticket_id: &str, name: &str, content: &[u8]) -> Result<ArtifactMeta, OrgaError> {
        let dest = self.artifact_path(ticket_id, &self.agent_name, name);
        let now = Utc::now();

        if self.remote.is_none() {
            let repo = self.open_repo()?;
            return commit_local(&repo, &dest, content, ticket_id, &self.agent_name, name, now);
        }

        let remote_name = self.remote.as_deref().unwrap();
        let mut last_err = String::new();

        for (attempt, &delay_ms) in RETRY_DELAYS_MS.iter().enumerate() {
            let repo = self.open_repo()?;

            if let Err(e) = fetch_rebase(&repo, remote_name, &self.branch, &self.auth) {
                last_err = format!("sync failed: {e}");
                if attempt < RETRY_DELAYS_MS.len() - 1 {
                    thread::sleep(Duration::from_millis(delay_ms));
                }
                continue;
            }

            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    OrgaError::BackendError(format!("cannot create artifact dir: {e}"))
                })?;
            }
            fs::write(&dest, content).map_err(|e| {
                OrgaError::BackendError(format!("cannot write artifact: {e}"))
            })?;

            match do_git_commit(&repo, &dest, ticket_id, &self.agent_name, name) {
                Err(e) => {
                    let _ = fs::remove_file(&dest);
                    last_err = e.to_string();
                    if attempt < RETRY_DELAYS_MS.len() - 1 {
                        thread::sleep(Duration::from_millis(delay_ms));
                    }
                    continue;
                }
                Ok(()) => {}
            }

            match push(&repo, remote_name, &self.branch, &self.auth) {
                Ok(()) => {
                    return Ok(ArtifactMeta {
                        ticket_id: ticket_id.to_string(),
                        agent_name: self.agent_name.clone(),
                        name: name.to_string(),
                        committed_at: now,
                    });
                }
                Err(e) => {
                    last_err = format!("push failed: {e}");
                    undo_commit(&repo);
                    let _ = fs::remove_file(&dest);
                    if attempt < RETRY_DELAYS_MS.len() - 1 {
                        thread::sleep(Duration::from_millis(delay_ms));
                    }
                }
            }
        }

        Err(OrgaError::BackendError(format!(
            "commit failed after {} attempts: {}",
            RETRY_DELAYS_MS.len(),
            last_err
        )))
    }

    fn get(&self, ticket_id: &str, name: &str) -> Result<Option<Artifact>, OrgaError> {
        if let Some(ref remote_name) = self.remote {
            let repo = self.open_repo()?;
            sync_with_fallback(&repo, remote_name, &self.branch, &self.auth, &self.logger);
        }

        let path = self.artifact_path(ticket_id, &self.agent_name, name);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path).map_err(|e| {
            OrgaError::BackendError(format!("cannot read artifact: {e}"))
        })?;
        Ok(Some(Artifact {
            meta: ArtifactMeta {
                ticket_id: ticket_id.to_string(),
                agent_name: self.agent_name.clone(),
                name: name.to_string(),
                committed_at: Utc::now(),
            },
            content,
        }))
    }

    fn list(&self, ticket_id: &str) -> Result<Vec<ArtifactMeta>, OrgaError> {
        if let Some(ref remote_name) = self.remote {
            let repo = self.open_repo()?;
            sync_with_fallback(&repo, remote_name, &self.branch, &self.auth, &self.logger);
        }

        let ticket_dir = self.repo_path.join("artifacts").join(ticket_id);
        if !ticket_dir.exists() {
            return Ok(vec![]);
        }

        let mut results = vec![];
        for agent_entry in read_dir_sorted(&ticket_dir)? {
            if !agent_entry.is_dir() {
                continue;
            }
            let agent_name = match agent_entry.file_name() {
                Some(n) => n.to_string_lossy().into_owned(),
                None => continue,
            };
            for artifact_entry in read_dir_sorted(&agent_entry)? {
                if artifact_entry.is_file() {
                    let name = match artifact_entry.file_name() {
                        Some(n) => n.to_string_lossy().into_owned(),
                        None => continue,
                    };
                    results.push(ArtifactMeta {
                        ticket_id: ticket_id.to_string(),
                        agent_name: agent_name.clone(),
                        name,
                        committed_at: Utc::now(),
                    });
                }
            }
        }
        Ok(results)
    }
}

fn read_dir_sorted(path: &Path) -> Result<Vec<PathBuf>, OrgaError> {
    let mut entries: Vec<PathBuf> = fs::read_dir(path)
        .map_err(|e| OrgaError::BackendError(format!("cannot read dir {}: {e}", path.display())))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    Ok(entries)
}

fn do_git_commit(repo: &Repository, dest: &Path, ticket_id: &str, agent_name: &str, name: &str) -> Result<(), OrgaError> {
    let mut index = repo.index().map_err(|e| OrgaError::BackendError(e.to_string()))?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .map_err(|e| OrgaError::BackendError(format!("git add failed: {e}")))?;
    index.write().map_err(|e| OrgaError::BackendError(e.to_string()))?;

    let sig = Signature::now(agent_name, &format!("{agent_name}@orga"))
        .map_err(|e| OrgaError::BackendError(e.to_string()))?;
    let tree_oid = index.write_tree().map_err(|e| OrgaError::BackendError(e.to_string()))?;
    let tree = repo.find_tree(tree_oid).map_err(|e| OrgaError::BackendError(e.to_string()))?;
    let message = format!("artifact({ticket_id}/{agent_name}): {name}");

    let parents: Vec<git2::Commit> = match repo.head() {
        Ok(head) => {
            let oid = head.target().ok_or_else(|| OrgaError::BackendError("HEAD has no target".into()))?;
            vec![repo.find_commit(oid).map_err(|e| OrgaError::BackendError(e.to_string()))?]
        }
        Err(_) => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &parent_refs)
        .map_err(|e| OrgaError::BackendError(format!("git commit failed: {e}")))?;

    let _ = dest;
    Ok(())
}

fn commit_local(repo: &Repository, dest: &Path, content: &[u8], ticket_id: &str, agent_name: &str, name: &str, now: chrono::DateTime<Utc>) -> Result<ArtifactMeta, OrgaError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            OrgaError::BackendError(format!("cannot create artifact dir: {e}"))
        })?;
    }
    fs::write(dest, content).map_err(|e| {
        OrgaError::BackendError(format!("cannot write artifact: {e}"))
    })?;
    do_git_commit(repo, dest, ticket_id, agent_name, name)?;
    Ok(ArtifactMeta {
        ticket_id: ticket_id.to_string(),
        agent_name: agent_name.to_string(),
        name: name.to_string(),
        committed_at: now,
    })
}

fn undo_commit(repo: &Repository) {
    if let Ok(head) = repo.head() {
        if let Some(head_oid) = head.target() {
            if let Ok(head_commit) = repo.find_commit(head_oid) {
                if head_commit.parent_count() > 0 {
                    if let Ok(parent) = head_commit.parent(0) {
                        let _ = repo.reset(parent.as_object(), ResetType::Hard, None);
                    }
                }
            }
        }
    }
}

fn sync_with_fallback(repo: &Repository, remote_name: &str, branch: &str, auth: &GitAuth, logger: &Logger) {
    for (attempt, &delay_ms) in RETRY_DELAYS_MS.iter().enumerate() {
        match fetch_rebase(repo, remote_name, branch, auth) {
            Ok(()) => return,
            Err(_) => {
                if attempt < RETRY_DELAYS_MS.len() - 1 {
                    thread::sleep(Duration::from_millis(delay_ms));
                }
            }
        }
    }
    logger.warn("artifact store sync failed, reading stale local data");
}

fn build_callbacks(auth: &GitAuth) -> RemoteCallbacks<'_> {
    let mut callbacks = RemoteCallbacks::new();
    let ssh_key = auth.ssh_key.clone();
    let ssh_passphrase = auth.ssh_passphrase.clone();
    let http_username = auth.http_username.clone();
    let http_password = auth.http_password.clone();

    callbacks.credentials(move |url, username_from_url, allowed| {
        if allowed.contains(git2::CredentialType::SSH_KEY) {
            let username = username_from_url.unwrap_or("git");
            if let Some(ref key_path) = ssh_key {
                return Cred::ssh_key(
                    username,
                    None,
                    key_path,
                    ssh_passphrase.as_deref(),
                );
            }
            return Cred::ssh_key_from_agent(username);
        }
        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            let user = http_username.as_deref().unwrap_or("");
            let pass = http_password.as_deref().unwrap_or("");
            return Cred::userpass_plaintext(user, pass);
        }
        // Let git2 fall through to default credential handling
        let _ = url;
        Err(git2::Error::from_str("no suitable credentials configured"))
    });

    callbacks
}

fn fetch_rebase(repo: &Repository, remote_name: &str, branch: &str, auth: &GitAuth) -> Result<(), git2::Error> {
    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.remote_callbacks(build_callbacks(auth));

    let mut remote = repo.find_remote(remote_name)?;
    remote.fetch(&[branch], Some(&mut fetch_opts), None)?;

    let remote_ref = format!("refs/remotes/{remote_name}/{branch}");
    let remote_oid = repo.refname_to_id(&remote_ref)?;
    let remote_commit = repo.find_annotated_commit(remote_oid)?;

    let head_oid = match repo.head()?.target() {
        Some(oid) => oid,
        None => return Err(git2::Error::from_str("HEAD has no target")),
    };

    let (analysis, _) = repo.merge_analysis(&[&remote_commit])?;

    if analysis.is_up_to_date() {
        // nothing to rebase
    } else if analysis.is_fast_forward() {
        let mut head_ref = repo.head()?;
        head_ref.set_target(remote_oid, "fast-forward")?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    } else {
        // Rebase: replay local commits on top of remote
        let local_commit = repo.find_commit(head_oid)?;
        let base_oid = repo.merge_base(head_oid, remote_oid)?;

        // Collect local commits since base
        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_oid)?;
        revwalk.hide(base_oid)?;
        let local_commits: Vec<git2::Oid> = revwalk.collect::<Result<_, _>>()?;
        let local_commits: Vec<git2::Commit> = local_commits
            .into_iter()
            .rev()
            .map(|oid| repo.find_commit(oid))
            .collect::<Result<_, _>>()?;

        // Reset to remote HEAD
        let remote_commit_obj = repo.find_commit(remote_oid)?;
        repo.reset(remote_commit_obj.as_object(), ResetType::Hard, None)?;

        // Replay each local commit
        for commit in &local_commits {
            let tree = commit.tree()?;
            let sig = commit.author();
            let msg = commit.message().unwrap_or("");

            let parent_oid = repo.head()?.target()
                .ok_or_else(|| git2::Error::from_str("HEAD has no target during rebase"))?;
            let parent = repo.find_commit(parent_oid)?;

            // Apply tree diff onto current HEAD
            let base_tree = if commit.parent_count() > 0 {
                Some(commit.parent(0)?.tree()?)
            } else {
                None
            };
            let current_tree = parent.tree()?;
            let diff = repo.diff_tree_to_tree(base_tree.as_ref(), Some(&tree), None)?;

            let mut index = repo.apply_to_tree(&current_tree, &diff, None)?;
            let new_tree_oid = index.write_tree_to(repo)?;
            let new_tree = repo.find_tree(new_tree_oid)?;

            repo.commit(Some("HEAD"), &sig, &sig, msg, &new_tree, &[&parent])?;

            let _ = local_commit; // suppress unused warning
        }
    }

    Ok(())
}

fn push(repo: &Repository, remote_name: &str, branch: &str, auth: &GitAuth) -> Result<(), git2::Error> {
    let mut push_opts = git2::PushOptions::new();
    push_opts.remote_callbacks(build_callbacks(auth));
    let mut push_remote = repo.find_remote(remote_name)?;
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    push_remote.push(&[&refspec], Some(&mut push_opts))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_repo(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        let sig = Signature::now("test", "test@test.com").unwrap();
        let tree_oid = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        {
            let tree = repo.find_tree(tree_oid).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();
        }
        repo
    }

    fn make_store(dir: &Path, agent: &str) -> GitArtifactStore {
        let logger = Arc::new(Logger::new(Path::new("/dev/null"), false));
        GitArtifactStore::new(
            dir.to_str().unwrap().to_string(),
            agent.to_string(),
            None,
            "main".to_string(),
            GitAuth::default(),
            logger,
        )
    }

    #[test]
    fn commit_creates_file_and_returns_meta() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        let store = make_store(tmp.path(), "agent-1");
        let meta = store.commit("TICKET-1", "report.md", b"hello world").unwrap();
        assert_eq!(meta.ticket_id, "TICKET-1");
        assert_eq!(meta.agent_name, "agent-1");
        assert_eq!(meta.name, "report.md");
        let path = tmp.path().join("artifacts/TICKET-1/agent-1/report.md");
        assert_eq!(fs::read_to_string(path).unwrap(), "hello world");
    }

    #[test]
    fn commit_overwrites_existing_artifact() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        let store = make_store(tmp.path(), "agent-1");
        store.commit("TICKET-1", "report.md", b"v1").unwrap();
        store.commit("TICKET-1", "report.md", b"v2").unwrap();
        let artifact = store.get("TICKET-1", "report.md").unwrap().unwrap();
        assert_eq!(artifact.content, "v2");
    }

    #[test]
    fn list_empty_when_no_artifacts() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        let store = make_store(tmp.path(), "agent-1");
        let results = store.list("TICKET-1").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn list_single_agent_artifacts() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        let store = make_store(tmp.path(), "agent-1");
        store.commit("TICKET-1", "report.md", b"r").unwrap();
        store.commit("TICKET-1", "output.json", b"{}").unwrap();
        let results = store.list("TICKET-1").unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|m| m.agent_name == "agent-1"));
    }

    #[test]
    fn list_multiple_agents() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        let store1 = make_store(tmp.path(), "agent-1");
        let store2 = make_store(tmp.path(), "agent-2");
        store1.commit("TICKET-1", "report.md", b"r1").unwrap();
        store2.commit("TICKET-1", "report.md", b"r2").unwrap();
        let results = store1.list("TICKET-1").unwrap();
        assert_eq!(results.len(), 2);
        let agents: Vec<&str> = results.iter().map(|m| m.agent_name.as_str()).collect();
        assert!(agents.contains(&"agent-1"));
        assert!(agents.contains(&"agent-2"));
    }

    #[test]
    fn get_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        let store = make_store(tmp.path(), "agent-1");
        assert!(store.get("TICKET-1", "report.md").unwrap().is_none());
    }

    #[test]
    fn get_returns_artifact_when_present() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        let store = make_store(tmp.path(), "agent-1");
        store.commit("TICKET-1", "notes.txt", b"my notes").unwrap();
        let artifact = store.get("TICKET-1", "notes.txt").unwrap().unwrap();
        assert_eq!(artifact.content, "my notes");
        assert_eq!(artifact.meta.name, "notes.txt");
        assert_eq!(artifact.meta.ticket_id, "TICKET-1");
    }

    #[test]
    fn list_no_remote_returns_local_data() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        let store = make_store(tmp.path(), "agent-1");
        store.commit("TICKET-1", "report.md", b"local").unwrap();
        let results = store.list("TICKET-1").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "report.md");
    }

    #[test]
    fn get_no_remote_returns_local_data() {
        let tmp = TempDir::new().unwrap();
        init_repo(tmp.path());
        let store = make_store(tmp.path(), "agent-1");
        store.commit("TICKET-1", "notes.txt", b"local content").unwrap();
        let artifact = store.get("TICKET-1", "notes.txt").unwrap().unwrap();
        assert_eq!(artifact.content, "local content");
    }

    #[test]
    fn commit_with_invalid_remote_cleans_up_and_errors() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());
        repo.remote("origin", "https://invalid.example.com/nonexistent.git").unwrap();
        drop(repo);

        let logger = Arc::new(Logger::new(Path::new("/dev/null"), false));
        let store = GitArtifactStore::new(
            tmp.path().to_str().unwrap().to_string(),
            "agent-1".to_string(),
            Some("origin".to_string()),
            "main".to_string(),
            GitAuth::default(),
            logger,
        );

        let head_before = {
            let r = Repository::open(tmp.path()).unwrap();
            r.head().unwrap().target().unwrap()
        };

        let result = store.commit("TICKET-1", "report.md", b"content");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed") || err.contains("error") || err.contains("commit"), "unexpected error: {err}");

        let artifact_path = tmp.path().join("artifacts/TICKET-1/agent-1/report.md");
        assert!(!artifact_path.exists(), "artifact file should be cleaned up after failed commit");

        let head_after = {
            let r = Repository::open(tmp.path()).unwrap();
            r.head().unwrap().target().unwrap()
        };
        assert_eq!(head_before, head_after, "HEAD should be unchanged after failed commit");
    }
}
