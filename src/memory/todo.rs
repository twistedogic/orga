use std::fs;
use std::path::Path;

use rusqlite::{Connection, params};

use crate::error::OrgaError;

pub struct TodoStore {
    conn: Connection,
}

impl TodoStore {
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
            "CREATE TABLE IF NOT EXISTS agent_todos (
                ticket_id  TEXT NOT NULL,
                scope      TEXT NOT NULL,
                todos      TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (ticket_id, scope)
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn set(&self, ticket_id: &str, scope: &str, todos: &str) -> Result<(), OrgaError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO agent_todos (ticket_id, scope, todos, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(ticket_id, scope) DO UPDATE SET todos = ?3, updated_at = ?4",
            params![ticket_id, scope, todos, now],
        )?;
        Ok(())
    }

    pub fn get(&self, ticket_id: &str, scope: &str) -> Result<Option<String>, OrgaError> {
        let mut stmt = self
            .conn
            .prepare("SELECT todos FROM agent_todos WHERE ticket_id = ?1 AND scope = ?2")?;
        let mut rows = stmt.query(params![ticket_id, scope])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }
}
