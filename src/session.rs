use anyhow::{anyhow, Context, Result};
use limbo::{Builder, Connection, Value};
use rand::{distributions::Alphanumeric, Rng};
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
    pub async fn new(path: &str) -> Result<Self> {
        let db = Builder::new_local(path)
            .build()
            .await
            .with_context(|| format!("Failed to open database at {}", path))?;
        let conn = db.connect()?;
        // Create sessions table if missing (with summary column)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (\
             id INTEGER PRIMARY KEY AUTOINCREMENT,\
             name TEXT NOT NULL,\
             messages TEXT NOT NULL,\
             summary TEXT)",
            (),
        )
        .await?;
        // Ensure summary column exists only if missing
        let mut has_summary = false;
        conn.clone().pragma_query("table_info(sessions)", |row| {
            if let Ok(Value::Text(col)) = row.get_value(1) {
                if col == "summary" {
                    has_summary = true;
                }
            }
            Ok(())
        })?;
        if !has_summary {
            // Attempt to add summary column; ignore duplicate-column errors
            if let Err(err) = conn
                .execute("ALTER TABLE sessions ADD COLUMN summary TEXT", ())
                .await
            {
                let msg = err.to_string();
                if !msg.contains("duplicate column name") {
                    return Err(err.into());
                }
            }
        }
        Ok(SessionManager { conn })
    }

    /// List sessions; returns (id, name, optional summary).
    pub async fn list_sessions(&self) -> Result<Vec<(i64, String, Option<String>)>> {
        // Try selecting with summary column
        let attempt = self
            .conn
            .query("SELECT id, name, summary FROM sessions", ())
            .await;
        if let Ok(mut rows) = attempt {
            let mut sessions = Vec::new();
            while let Some(row) = rows.next().await? {
                let id = match row.get_value(0)? {
                    Value::Integer(i) => i,
                    _ => continue,
                };
                let name = match row.get_value(1)? {
                    Value::Text(s) => s,
                    _ => continue,
                };
                let summary = match row.get_value(2)? {
                    Value::Text(s) if !s.is_empty() => Some(s),
                    _ => None,
                };
                sessions.push((id, name, summary));
            }
            Ok(sessions)
        } else {
            // Fallback if summary column missing
            let mut rows = self.conn.query("SELECT id, name FROM sessions", ()).await?;
            let mut sessions = Vec::new();
            while let Some(row) = rows.next().await? {
                let id = match row.get_value(0)? {
                    Value::Integer(i) => i,
                    _ => continue,
                };
                let name = match row.get_value(1)? {
                    Value::Text(s) => s,
                    _ => continue,
                };
                sessions.push((id, name, None));
            }
            Ok(sessions)
        }
    }

    pub async fn load_session(&self, id: i64) -> Result<Vec<SessionMessage>> {
        let mut rows = self
            .conn
            .query("SELECT messages FROM sessions WHERE id = ?", [id])
            .await?;
        if let Some(row) = rows.next().await? {
            let json = match row.get_value(0)? {
                Value::Text(s) => s,
                _ => String::new(),
            };
            let messages: Vec<SessionMessage> = serde_json::from_str(&json)
                .with_context(|| "Failed to deserialize session messages")?;
            Ok(messages)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn create_session(&self, name: &str, messages: &[SessionMessage]) -> Result<i64> {
        let json = serde_json::to_string(messages)
            .with_context(|| "Failed to serialize session messages")?;
        // Ensure unique session name: append random suffix if name already exists
        let mut final_name = name.to_string();
        let existing = self.list_sessions().await?;
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
        self.conn
            .execute(
                "INSERT INTO sessions (name, messages) VALUES (?, ?)",
                (final_name.as_str(), json.as_str()),
            )
            .await?;
        // Retrieve the new session id
        let mut rows = self
            .conn
            .query(
                "SELECT id FROM sessions WHERE name = ?",
                [final_name.as_str()],
            )
            .await?;
        if let Some(row) = rows.next().await? {
            let id = match row.get_value(0)? {
                Value::Integer(i) => i,
                _ => return Err(anyhow!("Invalid session id type")),
            };
            Ok(id)
        } else {
            Err(anyhow!("Failed to retrieve session id"))
        }
    }

    pub async fn update_session(&self, id: i64, messages: &[SessionMessage]) -> Result<()> {
        let json = serde_json::to_string(messages)
            .with_context(|| "Failed to serialize session messages")?;
        self.conn
            .execute(
                "UPDATE sessions SET messages = ? WHERE id = ?",
                (json.as_str(), id),
            )
            .await?;
        Ok(())
    }

    /// Update summary field for a session
    pub async fn update_summary(&self, id: i64, summary: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE sessions SET summary = ? WHERE id = ?",
                (summary, id),
            )
            .await?;
        Ok(())
    }
    /// Delete a session by id
    pub async fn delete_session(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?", [id])
            .await?;
        Ok(())
    }
}
