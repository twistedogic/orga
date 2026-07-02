use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use crate::error::OrgaError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompactionRecord {
    pub ticket_id: String,
    pub summary: String,
    pub compacted_through: DateTime<Utc>,
    pub compacted_count: usize,
    pub updated_at: DateTime<Utc>,
}

pub struct CompactionStore {
    conn: Connection,
}

impl CompactionStore {
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
            "CREATE TABLE IF NOT EXISTS comment_compaction (
                ticket_id         TEXT PRIMARY KEY,
                summary           TEXT NOT NULL,
                compacted_through TEXT NOT NULL,
                compacted_count   INTEGER NOT NULL,
                updated_at        TEXT NOT NULL
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn set(
        &self,
        ticket_id: &str,
        summary: &str,
        compacted_through: DateTime<Utc>,
        compacted_count: usize,
    ) -> Result<(), OrgaError> {
        let now = chrono::Utc::now().to_rfc3339();
        let through = compacted_through.to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO comment_compaction
             (ticket_id, summary, compacted_through, compacted_count, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![ticket_id, summary, through, compacted_count as i64, now],
        )?;
        Ok(())
    }

    pub fn get(&self, ticket_id: &str) -> Result<Option<CompactionRecord>, OrgaError> {
        let mut stmt = self.conn.prepare(
            "SELECT ticket_id, summary, compacted_through, compacted_count, updated_at
             FROM comment_compaction WHERE ticket_id = ?1",
        )?;
        let mut rows = stmt.query(params![ticket_id])?;
        if let Some(row) = rows.next()? {
            let through_str: String = row.get(2)?;
            let updated_str: String = row.get(4)?;
            let compacted_through = through_str
                .parse::<DateTime<Utc>>()
                .map_err(|e| OrgaError::BackendError(format!("invalid compacted_through: {e}")))?;
            let updated_at = updated_str
                .parse::<DateTime<Utc>>()
                .map_err(|e| OrgaError::BackendError(format!("invalid updated_at: {e}")))?;
            Ok(Some(CompactionRecord {
                ticket_id: row.get(0)?,
                summary: row.get(1)?,
                compacted_through,
                compacted_count: row.get::<_, i64>(3)? as usize,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete(&self, ticket_id: &str) -> Result<(), OrgaError> {
        self.conn.execute(
            "DELETE FROM comment_compaction WHERE ticket_id = ?1",
            params![ticket_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn open_temp_compaction_store() -> CompactionStore {
        let dir = tempdir().unwrap();
        let path = dir.keep().join("memory.db");
        CompactionStore::open(&path).unwrap()
    }

    #[test]
    fn compaction_set_and_get_roundtrip() {
        let store = open_temp_compaction_store();
        let ts = chrono::DateTime::parse_from_rfc3339("2024-03-01T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        store.set("T-1", "summary text", ts, 10).unwrap();
        let rec = store.get("T-1").unwrap().unwrap();
        assert_eq!(rec.ticket_id, "T-1");
        assert_eq!(rec.summary, "summary text");
        assert_eq!(rec.compacted_count, 10);
        assert_eq!(rec.compacted_through, ts);
    }

    #[test]
    fn compaction_overwrite_replaces_record() {
        let store = open_temp_compaction_store();
        let ts1 = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let ts2 = chrono::DateTime::parse_from_rfc3339("2024-06-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        store.set("T-1", "old summary", ts1, 5).unwrap();
        store.set("T-1", "new summary", ts2, 20).unwrap();
        let rec = store.get("T-1").unwrap().unwrap();
        assert_eq!(rec.summary, "new summary");
        assert_eq!(rec.compacted_count, 20);
        assert_eq!(rec.compacted_through, ts2);
    }

    #[test]
    fn compaction_delete_removes_record() {
        let store = open_temp_compaction_store();
        let ts = chrono::DateTime::parse_from_rfc3339("2024-03-01T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        store.set("T-1", "summary", ts, 5).unwrap();
        store.delete("T-1").unwrap();
        assert!(store.get("T-1").unwrap().is_none());
    }

    #[test]
    fn compaction_delete_non_existent_is_noop() {
        let store = open_temp_compaction_store();
        store.delete("NONEXISTENT").unwrap();
    }

    #[test]
    fn compaction_get_missing_returns_none() {
        let store = open_temp_compaction_store();
        assert!(store.get("MISSING").unwrap().is_none());
    }
}
