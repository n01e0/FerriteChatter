use anyhow::{Context, Result};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SessionFile {
    name: String,
    summary: Option<String>,
    messages: Vec<SessionMessage>,
}

pub struct SessionManager {
    sessions_dir: PathBuf,
}

impl SessionManager {
    /// Initialize the session manager, creating the sessions directory if needed.
    pub fn new() -> Result<Self> {
        let home = env::var("HOME").context("HOME environment variable not set")?;
        let config_base = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{home}/.config"));
        let sessions_dir = Path::new(&config_base).join("ferrite").join("sessions");
        fs::create_dir_all(&sessions_dir)
            .with_context(|| format!("Failed to create sessions directory at {sessions_dir:?}"))?;
        Ok(SessionManager { sessions_dir })
    }

    /// List sessions; returns (id, name, optional summary).
    /// List available sessions: returns (id, name, optional summary).
    pub fn list_sessions(&self) -> Result<Vec<(i64, String, Option<String>)>> {
        let entries = fs::read_dir(&self.sessions_dir).with_context(|| {
            format!(
                "Failed to read sessions directory at {:?}",
                self.sessions_dir
            )
        })?;
        let mut sessions = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .context("Invalid session file name")?;
            let id: i64 = file_stem.parse().with_context(|| {
                format!("Failed to parse session id from file name {file_stem}")
            })?;
            let mut file = fs::File::open(&path)
                .with_context(|| format!("Failed to open session file {path:?}"))?;
            let mut content = String::new();
            file.read_to_string(&mut content)
                .with_context(|| format!("Failed to read session file {path:?}"))?;
            let session_file: SessionFile = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse JSON in {path:?}"))?;
            sessions.push((id, session_file.name, session_file.summary));
        }
        sessions.sort_by_key(|(id, _, _)| *id);
        Ok(sessions)
    }

    /// Load messages for a session by id.
    pub fn load_session(&self, id: i64) -> Result<Vec<SessionMessage>> {
        let path = self.sessions_dir.join(format!("{id}.json"));
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read session file {path:?}"))?;
        let session_file: SessionFile = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON in {path:?}"))?;
        Ok(session_file.messages)
    }

    /// Create a new session with given name and messages. Returns new session id.
    pub fn create_session(&self, name: &str, messages: &[SessionMessage]) -> Result<i64> {
        let sessions = self.list_sessions()?;
        let existing_names: Vec<String> = sessions.iter().map(|(_, n, _)| n.clone()).collect();
        let mut final_name = name.to_string();
        if existing_names.contains(&final_name) {
            let mut rng = rand::rng();
            loop {
                let suffix: String = Alphanumeric.sample_string(&mut rng, 6);
                let candidate = format!("{name}-{suffix}");
                if !existing_names.contains(&candidate) {
                    final_name = candidate;
                    break;
                }
            }
        }
        let new_id = sessions.iter().map(|(id, _, _)| *id).max().unwrap_or(0) + 1;
        let session_file = SessionFile {
            name: final_name,
            summary: None,
            messages: messages.to_vec(),
        };
        let serialized = serde_json::to_string(&session_file)
            .with_context(|| "Failed to serialize session to JSON")?;
        let path = self.sessions_dir.join(format!("{new_id}.json"));
        fs::write(&path, serialized)
            .with_context(|| format!("Failed to write session file {path:?}"))?;
        Ok(new_id)
    }

    /// Update the messages for an existing session.
    pub fn update_session(&self, id: i64, messages: &[SessionMessage]) -> Result<()> {
        let path = self.sessions_dir.join(format!("{id}.json"));
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read session file {path:?}"))?;
        let mut session_file: SessionFile = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON in {path:?}"))?;
        session_file.messages = messages.to_vec();
        let serialized = serde_json::to_string(&session_file)
            .with_context(|| "Failed to serialize session to JSON")?;
        fs::write(&path, serialized)
            .with_context(|| format!("Failed to write session file {path:?}"))?;
        Ok(())
    }

    /// Update the summary for an existing session.
    pub fn update_summary(&self, id: i64, summary: &str) -> Result<()> {
        let path = self.sessions_dir.join(format!("{id}.json"));
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read session file {path:?}"))?;
        let mut session_file: SessionFile = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON in {path:?}"))?;
        session_file.summary = Some(summary.to_string());
        let serialized = serde_json::to_string(&session_file)
            .with_context(|| "Failed to serialize session to JSON")?;
        fs::write(&path, serialized)
            .with_context(|| format!("Failed to write session file {path:?}"))?;
        Ok(())
    }
    /// Delete a session by id.
    pub fn delete_session(&self, id: i64) -> Result<()> {
        let path = self.sessions_dir.join(format!("{id}.json"));
        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete session file {path:?}"))?;
        Ok(())
    }
}
