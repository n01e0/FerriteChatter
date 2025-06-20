use anyhow::{Context, Result};
use rand::{distributions::Alphanumeric, Rng};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
}

pub struct SessionManager {
    conn: Connection,
}

impl SessionManager {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database at {}", path))?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        // Configure WAL mode and synchronization; allow waiting for locks
        conn.pragma_update(None, "journal_mode", &"WAL")?;
        conn.pragma_update(None, "synchronous", &"NORMAL")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 messages TEXT NOT NULL,
                 summary TEXT
             )",
            [],
        )?;
        Ok(SessionManager { conn })
    }

    /// List sessions; returns (id, name, optional summary).
    pub fn list_sessions(&self) -> Result<Vec<(i64, String, Option<String>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, summary FROM sessions")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let summary_raw: Option<String> = row.get(2)?;
            let summary = match summary_raw {
                Some(s) if !s.is_empty() => Some(s),
                _ => None,
            };
            Ok((id, name, summary))
        })?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    pub fn load_session(&self, id: i64) -> Result<Vec<SessionMessage>> {
        let mut stmt = self
            .conn
            .prepare("SELECT messages FROM sessions WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            let messages: Vec<SessionMessage> = serde_json::from_str(&json)
                .with_context(|| "Failed to deserialize session messages")?;
            Ok(messages)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn create_session(&self, name: &str, messages: &[SessionMessage]) -> Result<i64> {
        let json = serde_json::to_string(messages)
            .with_context(|| "Failed to serialize session messages")?;
        // Ensure unique session name: append random suffix if name already exists
        let mut final_name = name.to_string();
        let existing = self.list_sessions()?;
        if existing
            .iter()
            .any(|(_, session_name, _)| session_name == &final_name)
        {
            let mut rng = rand::thread_rng();
            loop {
                let suffix: String = (&mut rng)
                    .sample_iter(&Alphanumeric)
                    .take(6)
                    .map(char::from)
                    .collect();
                let candidate = format!("{}-{}", name, suffix);
                if !existing
                    .iter()
                    .any(|(_, session_name, _)| session_name == &candidate)
                {
                    final_name = candidate;
                    break;
                }
            }
        }
        // Insert the session with the (possibly modified) unique name
        self.conn.execute(
            "INSERT INTO sessions (name, messages) VALUES (?1, ?2)",
            params![final_name, json],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_session(&self, id: i64, messages: &[SessionMessage]) -> Result<()> {
        let json = serde_json::to_string(messages)
            .with_context(|| "Failed to serialize session messages")?;
        self.conn.execute(
            "UPDATE sessions SET messages = ?1 WHERE id = ?2",
            params![json, id],
        )?;
        Ok(())
    }

    /// Update summary field for a session
    pub fn update_summary(&self, id: i64, summary: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET summary = ?1 WHERE id = ?2",
            params![summary, id],
        )?;
        Ok(())
    }
    /// Delete a session by id
    pub fn delete_session(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }
}
