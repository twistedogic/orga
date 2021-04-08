use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};

use crate::error::OrgaError;

pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    pub fn open(db_path: &Path) -> Result<Self, OrgaError> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                OrgaError::BackendError(format!(
                    "cannot create memory dir {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memory (
                ticket_id  TEXT PRIMARY KEY,
                context    TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn set(&self, ticket_id: &str, context: &str) -> Result<(), OrgaError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO memory (ticket_id, context, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(ticket_id) DO UPDATE SET context = ?2, updated_at = ?3",
            params![ticket_id, context, now],
        )?;
        Ok(())
    }

    pub fn get(&self, ticket_id: &str) -> Result<Option<MemoryEntry>, OrgaError> {
        let mut stmt = self.conn.prepare(
            "SELECT ticket_id, context, updated_at FROM memory WHERE ticket_id = ?1",
        )?;
        let mut rows = stmt.query(params![ticket_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(MemoryEntry {
                ticket_id: row.get(0)?,
                context: row.get(1)?,
                updated_at: row.get(2)?,
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct MemoryEntry {
    pub ticket_id: String,
    pub context: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn open_temp_store() -> MemoryStore {
        let dir = tempdir().unwrap();
        let path = dir.keep().join("memory.db");
        MemoryStore::open(&path).unwrap()
    }

    #[test]
    fn set_and_get_returns_context() {
        let store = open_temp_store();
        store.set("TICKET-1", "working on auth module").unwrap();
        let entry = store.get("TICKET-1").unwrap().unwrap();
        assert_eq!(entry.context, "working on auth module");
        assert_eq!(entry.ticket_id, "TICKET-1");
    }

    #[test]
    fn get_missing_returns_none() {
        let store = open_temp_store();
        let result = store.get("NONEXISTENT").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn set_overwrites_previous_value() {
        let store = open_temp_store();
        store.set("TICKET-2", "first context").unwrap();
        store.set("TICKET-2", "updated context").unwrap();
        let entry = store.get("TICKET-2").unwrap().unwrap();
        assert_eq!(entry.context, "updated context");
    }

    #[test]
    fn auto_creates_db_file() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("nested").join("memory.db");
        assert!(!db_path.exists());
        let store = MemoryStore::open(&db_path).unwrap();
        store.set("T-1", "hello").unwrap();
        assert!(db_path.exists());
    }
}
