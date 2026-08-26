use std::path::{Component, Path, PathBuf};

use crate::error::OrgaError;

pub struct WorkspaceStore {
    base: PathBuf,
}

impl WorkspaceStore {
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    fn sanitize_ticket_id(ticket_id: &str) -> String {
        ticket_id
            .chars()
            .map(|c| match c {
                '/' | ':' | '\\' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c => c,
            })
            .collect()
    }

    fn ticket_root(&self, ticket_id: &str) -> PathBuf {
        self.base.join(Self::sanitize_ticket_id(ticket_id))
    }

    pub fn ticket_root_path(&self, ticket_id: &str) -> PathBuf {
        self.ticket_root(ticket_id)
    }

    fn safe_path(&self, ticket_id: &str, path: &str) -> Result<PathBuf, OrgaError> {
        let root = self.ticket_root(ticket_id);
        let joined = normalize_path(&root.join(path));
        if !joined.starts_with(&root) {
            return Err(OrgaError::BackendError(
                "path escapes workspace root".to_string(),
            ));
        }
        Ok(joined)
    }

    pub fn read(&self, ticket_id: &str, path: &str) -> Result<String, OrgaError> {
        let full = self.safe_path(ticket_id, path)?;
        let bytes = std::fs::read(&full).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                OrgaError::NotFound("file not found".to_string())
            } else {
                OrgaError::BackendError(format!("read error: {e}"))
            }
        })?;
        String::from_utf8(bytes)
            .map_err(|_| OrgaError::BackendError("file contains binary content".to_string()))
    }

    pub fn write(&self, ticket_id: &str, path: &str, content: &str) -> Result<(), OrgaError> {
        let root = self.ticket_root(ticket_id);
        let full = normalize_path(&root.join(path));
        if !full.starts_with(&root) {
            return Err(OrgaError::BackendError(
                "path escapes workspace root".to_string(),
            ));
        }
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| OrgaError::BackendError(format!("create dirs error: {e}")))?;
        }
        std::fs::write(&full, content)
            .map_err(|e| OrgaError::BackendError(format!("write error: {e}")))
    }

    pub fn list(&self, ticket_id: &str) -> Result<String, OrgaError> {
        let root = self.ticket_root(ticket_id);
        if !root.exists() {
            return Ok(String::new());
        }
        let mut paths: Vec<String> = Vec::new();
        visit_dir(&root, &root, &mut paths)
            .map_err(|e| OrgaError::BackendError(format!("list error: {e}")))?;
        paths.sort();
        Ok(paths.join("\n"))
    }
}

/// Render a relative path with `/` separators on every platform.
///
/// Relative paths in `orga` are logical identifiers, not host paths: they are
/// memory keys passed back to `ContextRepository::read`/`delete`, git index
/// paths, and listings shown to the language model. `Path::to_string_lossy`
/// emits `\` on Windows, so those identifiers would stop matching the
/// `/`-separated paths callers wrote.
pub fn to_slash(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            c => out.push(c),
        }
    }
    out
}

fn visit_dir(root: &Path, dir: &Path, paths: &mut Vec<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let path = entry.path();
        if ft.is_dir() {
            visit_dir(root, &path, paths)?;
        } else if ft.is_file()
            && let Ok(rel) = path.strip_prefix(root)
        {
            paths.push(to_slash(rel));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store(tmp: &TempDir) -> WorkspaceStore {
        WorkspaceStore::new(tmp.path().to_path_buf())
    }

    #[test]
    fn sanitize_ticket_id_replaces_unsafe_chars() {
        assert_eq!(
            WorkspaceStore::sanitize_ticket_id("PROJ-123/sub:task"),
            "PROJ-123_sub_task"
        );
        assert_eq!(WorkspaceStore::sanitize_ticket_id("TICKET-1"), "TICKET-1");
    }

    #[test]
    fn write_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let ws = store(&tmp);
        ws.write("T-1", "notes.md", "hello").unwrap();
        assert_eq!(ws.read("T-1", "notes.md").unwrap(), "hello");
    }

    #[test]
    fn write_creates_intermediate_dirs() {
        let tmp = TempDir::new().unwrap();
        let ws = store(&tmp);
        ws.write("T-1", "a/b/c/file.txt", "data").unwrap();
        assert_eq!(ws.read("T-1", "a/b/c/file.txt").unwrap(), "data");
    }

    #[test]
    fn read_missing_file_returns_not_found() {
        let tmp = TempDir::new().unwrap();
        let ws = store(&tmp);
        let err = ws.read("T-1", "missing.txt").unwrap_err();
        assert!(err.to_string().contains("not found"), "got: {err}");
    }

    #[test]
    fn path_traversal_rejected() {
        let tmp = TempDir::new().unwrap();
        let ws = store(&tmp);
        let err = ws.read("T-1", "../../etc/passwd").unwrap_err();
        assert!(err.to_string().contains("path escapes"), "got: {err}");
    }

    #[test]
    fn path_traversal_on_write_rejected() {
        let tmp = TempDir::new().unwrap();
        let ws = store(&tmp);
        let err = ws.write("T-1", "../../evil.txt", "bad").unwrap_err();
        assert!(err.to_string().contains("path escapes"), "got: {err}");
    }

    #[test]
    fn list_returns_flat_sorted_paths() {
        let tmp = TempDir::new().unwrap();
        let ws = store(&tmp);
        ws.write("T-1", "b.txt", "2").unwrap();
        ws.write("T-1", "a/c.txt", "3").unwrap();
        ws.write("T-1", "a.txt", "1").unwrap();
        let listing = ws.list("T-1").unwrap();
        let lines: Vec<&str> = listing.lines().collect();
        assert_eq!(lines, vec!["a.txt", "a/c.txt", "b.txt"]);
    }

    #[test]
    fn to_slash_uses_forward_separators_for_nested_paths() {
        let rel = PathBuf::from("a").join("b").join("c.txt");
        assert_eq!(to_slash(&rel), "a/b/c.txt");
    }

    #[test]
    fn list_empty_workspace_returns_empty_string() {
        let tmp = TempDir::new().unwrap();
        let ws = store(&tmp);
        assert_eq!(ws.list("T-1").unwrap(), "");
    }

    #[test]
    fn binary_file_read_returns_error() {
        let tmp = TempDir::new().unwrap();
        let ws = store(&tmp);
        let root = tmp.path().join("T-1");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("bin.dat"), [0u8, 159u8, 146u8, 150u8]).unwrap();
        let err = ws.read("T-1", "bin.dat").unwrap_err();
        assert!(err.to_string().contains("binary content"), "got: {err}");
    }
}
