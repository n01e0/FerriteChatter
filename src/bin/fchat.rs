use anyhow::{Context, Result};
use clap::Parser;
use inquire::{Confirm, Editor, Select, Text};
use openai::{
    chat::{ChatCompletion, ChatCompletionDelta, ChatCompletionMessage, ChatCompletionMessageRole},
    Credentials,
};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use FerriteChatter::{
    config::Config,
    core::{ask, Model, DEFAULT_MODEL},
    session::{SessionManager, SessionMessage},
};

/// Generate a one-sentence summary for a session via ChatCompletion
async fn generate_summary(
    session_msgs: &[SessionMessage],
    credentials: Credentials,
    model: &str,
) -> anyhow::Result<String> {
    let mut messages: Vec<ChatCompletionMessage> = Vec::new();
    messages.push(ChatCompletionMessage {
        role: ChatCompletionMessageRole::System,
        content: Some(
            "Please summarize the following conversation in one concise sentence:".to_string(),
        ),
        ..Default::default()
    });
    for m in session_msgs {
        let role = match m.role.as_str() {
            "assistant" => ChatCompletionMessageRole::Assistant,
            _ => ChatCompletionMessageRole::User,
        };
        messages.push(ChatCompletionMessage {
            role,
            content: Some(m.content.clone()),
            ..Default::default()
        });
    }
    let completion = ChatCompletion::builder(model, messages)
        .credentials(credentials.clone())
        .create()
        .await
        .with_context(|| "Failed to generate summary")?;
    let summary = completion
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();
    Ok(summary)
}

const SEED_PROMPT: &'static str = r#"
You are an engineer's assistant.
The user can reset the current state of the chat by inputting '/reset'.
The user can activate the editor by entering 'v', allowing them to input multiple lines of prompts.
To terminate, the user needs to input "exit".
"#;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Open Prompt(General Prompt)
    #[clap(long = "general", short = 'g')]
    general: Option<String>,
    /// OpenAI API Key
    #[clap(long = "key", short = 'k')]
    key: Option<String>,
    /// OpenAI API Base URL
    #[clap(long = "base-url", short = 'b')]
    base_url: Option<String>,
    /// OpenAI Model
    #[clap(long = "model", short = 'm', value_enum)]
    model: Option<Model>,
    /// Initial context file
    #[clap(long = "file", short = 'f')]
    file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = Config::load()?;

    let key = args.key.unwrap_or(
        config.get_openai_api_key().clone().unwrap_or(
            env::var("OPENAI_API_KEY")
                .with_context(|| "You need to set API key to the `OPENAI_API_KEY`")?,
        ),
    );
    let base_url = args
        .base_url
        .unwrap_or(config.get_openai_base_url().clone().unwrap_or(
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
        ));
    let credentials = Credentials::new(key, base_url);
    let model = args
        .model
        .unwrap_or(config.get_default_model().clone().unwrap_or(DEFAULT_MODEL))
        .as_str();

    let role = if !model.starts_with("o1") {
        ChatCompletionMessageRole::System
    } else {
        ChatCompletionMessageRole::User
    };

    let general_content = args.general.clone().unwrap_or(String::from(SEED_PROMPT));
    let file_path = args.file.clone();

    // Use XDG_CONFIG_HOME or fallback to $HOME/.config for ferrite data
    let home = env::var("HOME").with_context(|| "Where is the HOME?")?;
    let config_base = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));
    let ferrite_dir = Path::new(&config_base).join("ferrite");
    fs::create_dir_all(&ferrite_dir)?;
    let db_path = ferrite_dir.join("session.db").to_string_lossy().to_string();
    let session_manager = SessionManager::new(&db_path).await?;

    let existing_sessions = session_manager.list_sessions().await?;
    let mut session_id: i64;
    let mut messages: Vec<ChatCompletionMessage> = Vec::new();
    if existing_sessions.is_empty() {
        // No existing sessions: create a new one
        let name = Text::new("No sessions found. Enter a name for a new session:").prompt()?;
        messages.push(ChatCompletionMessage {
            role,
            content: Some(general_content.clone()),
            ..Default::default()
        });
        if let Some(path) = &file_path {
            let mut input_file = String::new();
            let _ = File::open(path)?.read_to_string(&mut input_file);
            messages.push(ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some(input_file),
                ..Default::default()
            });
        }
        let session_msgs: Vec<SessionMessage> = messages
            .iter()
            .map(|m| SessionMessage {
                role: match m.role {
                    ChatCompletionMessageRole::System => "system".to_string(),
                    ChatCompletionMessageRole::User => "user".to_string(),
                    ChatCompletionMessageRole::Assistant => "assistant".to_string(),
                    _ => "user".to_string(),
                },
                content: m.content.clone().unwrap_or_default(),
            })
            .collect();
        session_id = session_manager.create_session(&name, &session_msgs).await?;
    } else {
        // Session selection with inline preview in labels
        session_id = loop {
            let existing = session_manager.list_sessions().await?;
            if existing.is_empty() {
                // No sessions: create a new one
                let name =
                    Text::new("No sessions found. Enter a name for a new session:").prompt()?;
                messages.push(ChatCompletionMessage {
                    role,
                    content: Some(general_content.clone()),
                    ..Default::default()
                });
                if let Some(path) = &file_path {
                    let mut input_file = String::new();
                    let _ = File::open(path)?.read_to_string(&mut input_file);
                    messages.push(ChatCompletionMessage {
                        role: ChatCompletionMessageRole::User,
                        content: Some(input_file),
                        ..Default::default()
                    });
                }
                let session_msgs: Vec<SessionMessage> = messages
                    .iter()
                    .map(|m| SessionMessage {
                        role: match m.role {
                            ChatCompletionMessageRole::System => "system".to_string(),
                            ChatCompletionMessageRole::User => "user".to_string(),
                            ChatCompletionMessageRole::Assistant => "assistant".to_string(),
                            _ => "user".to_string(),
                        },
                        content: m.content.clone().unwrap_or_default(),
                    })
                    .collect();
                break session_manager.create_session(&name, &session_msgs).await?;
            }
            // Build labels and ids with summary preview
            let mut labels = Vec::new();
            let mut ids = Vec::new();
            for (id, name, summary_opt) in existing.iter() {
                let summary = if let Some(s) = summary_opt {
                    s.clone()
                } else {
                    String::new()
                };
                labels.push(if summary.is_empty() {
                    name.clone()
                } else {
                    format!("{} | {}", name, summary)
                });
                ids.push(*id);
            }
            labels.push("New session".to_string());
            labels.push("Delete session".to_string());
            let selection = Select::new("Choose a session:", labels.clone()).prompt()?;
            if selection == "New session" {
                let name = Text::new("Enter a name for a new session:").prompt()?;
                messages.push(ChatCompletionMessage {
                    role,
                    content: Some(general_content.clone()),
                    ..Default::default()
                });
                if let Some(path) = &file_path {
                    let mut input_file = String::new();
                    let _ = File::open(path)?.read_to_string(&mut input_file);
                    messages.push(ChatCompletionMessage {
                        role: ChatCompletionMessageRole::User,
                        content: Some(input_file),
                        ..Default::default()
                    });
                }
                let session_msgs: Vec<SessionMessage> = messages
                    .iter()
                    .map(|m| SessionMessage {
                        role: match m.role {
                            ChatCompletionMessageRole::System => "system".to_string(),
                            ChatCompletionMessageRole::User => "user".to_string(),
                            ChatCompletionMessageRole::Assistant => "assistant".to_string(),
                            _ => "user".to_string(),
                        },
                        content: m.content.clone().unwrap_or_default(),
                    })
                    .collect();
                break session_manager.create_session(&name, &session_msgs).await?;
            } else if selection == "Delete session" {
                // Select session to delete
                let names: Vec<String> = existing.iter().map(|(_, name, _)| name.clone()).collect();
                let to_delete = Select::new("Select session to delete:", names).prompt()?;
                if let Some((del_id, _, _)) = existing.iter().find(|(_, n, _)| n == &to_delete) {
                    if Confirm::new(&format!("Delete session '{}' ?", to_delete))
                        .with_default(false)
                        .prompt()?
                    {
                        session_manager.delete_session(*del_id).await?;
                        println!("Deleted session: {}", to_delete);
                    }
                }
                continue;
            } else {
                // Switch to selected session
                let idx = labels.iter().position(|l| l == &selection).unwrap();
                break ids[idx];
            }
        };
        // Load and populate messages for selected session
        messages.clear();
        let loaded = session_manager.load_session(session_id).await?;
        for m in loaded {
            let role_enum = match m.role.as_str() {
                "system" => ChatCompletionMessageRole::System,
                "assistant" => ChatCompletionMessageRole::Assistant,
                _ => ChatCompletionMessageRole::User,
            };
            messages.push(ChatCompletionMessage {
                role: role_enum,
                content: Some(m.content),
                ..Default::default()
            });
        }
    }

    let mut initial_state = messages.clone();

    loop {
        let input = Text::new("").prompt()?;
        match &input[..] {
            "exit" => {
                println!("Bye!");
                return Ok(());
            }
            "/reset" => {
                messages = Vec::from(&initial_state[..]);
            }
            "v" => {
                let input = Editor::new("Prompt:").prompt()?;
                messages.push(ChatCompletionMessage {
                    role: ChatCompletionMessageRole::User,
                    content: Some(input),
                    ..Default::default()
                });
                // save user message
                let session_msgs: Vec<SessionMessage> = messages
                    .iter()
                    .map(|m| SessionMessage {
                        role: match m.role {
                            ChatCompletionMessageRole::System => "system".to_string(),
                            ChatCompletionMessageRole::User => "user".to_string(),
                            ChatCompletionMessageRole::Assistant => "assistant".to_string(),
                            _ => "user".to_string(),
                        },
                        content: m.content.clone().unwrap_or_default(),
                    })
                    .collect();
                session_manager
                    .update_session(session_id, &session_msgs)
                    .await?;
                let stream = ChatCompletionDelta::builder(model, messages.clone())
                    .credentials(credentials.clone())
                    .create_stream()
                    .await
                    .with_context(|| "Can't open Stream")?;

                let answer = ask(stream)
                    .await?
                    .choices
                    .first()
                    .with_context(|| "Can't get choices")?
                    .message
                    .clone();
                messages.push(answer);
                // save assistant response
                let session_msgs: Vec<SessionMessage> = messages
                    .iter()
                    .map(|m| SessionMessage {
                        role: match m.role {
                            ChatCompletionMessageRole::System => "system".to_string(),
                            ChatCompletionMessageRole::User => "user".to_string(),
                            ChatCompletionMessageRole::Assistant => "assistant".to_string(),
                            _ => "user".to_string(),
                        },
                        content: m.content.clone().unwrap_or_default(),
                    })
                    .collect();
                session_manager
                    .update_session(session_id, &session_msgs)
                    .await?;
            }
            "/save" => {
                let path = Text::new("path:").prompt()?;
                let context = messages
                    .clone()
                    .into_iter()
                    .filter(|m| m.role != ChatCompletionMessageRole::System)
                    .filter_map(|m| {
                        if m.role == ChatCompletionMessageRole::Assistant {
                            m.content.map(|c| format!("Assistant:{}", c))
                        } else {
                            m.content
                        }
                    })
                    .collect::<Vec<String>>()
                    .join("\n");
                let mut out = File::create(path)?;
                out.write_all(context.as_bytes())?;
                let exit = Confirm::new("Context successfully saved!\nexit?[y/n]:")
                    .with_default(false)
                    .prompt()?;
                if exit {
                    println!("Bye!");
                    return Ok(());
                }
            }
            "/session" => {
                let existing_sessions = session_manager.list_sessions().await?;
                if existing_sessions.is_empty() {
                    println!("No sessions available.");
                } else {
                    // Inline preview labels for session switching
                    let mut labels = Vec::new();
                    let mut ids = Vec::new();
                    for (id, name, summary_opt) in &existing_sessions {
                        // Use or generate summary preview
                        let summary = if let Some(s) = summary_opt {
                            s.clone()
                        } else {
                            let msgs = session_manager.load_session(*id).await?;
                            let s = generate_summary(&msgs, credentials.clone(), model).await?;
                            session_manager.update_summary(*id, &s).await?;
                            s
                        };
                        labels.push(format!("{} | {}", name, summary));
                        ids.push(*id);
                    }
                    let selection = Select::new("Choose a session:", labels.clone()).prompt()?;
                    // Find selected id and load messages
                    if let Some(idx) = labels.iter().position(|l| l == &selection) {
                        let sel_id = ids[idx];
                        let loaded = session_manager.load_session(sel_id).await?;
                        messages.clear();
                        for m in loaded {
                            let role_enum = match m.role.as_str() {
                                "system" => ChatCompletionMessageRole::System,
                                "assistant" => ChatCompletionMessageRole::Assistant,
                                _ => ChatCompletionMessageRole::User,
                            };
                            messages.push(ChatCompletionMessage {
                                role: role_enum,
                                content: Some(m.content),
                                ..Default::default()
                            });
                        }
                        session_id = sel_id;
                        println!(
                            "Switched to session: {}",
                            selection.split(" | ").next().unwrap_or(&selection)
                        );
                        initial_state = messages.clone();
                    }
                }
            }
            "/history" => {
                // Print current session history
                for (_, m) in messages.iter().enumerate() {
                    let role_str = match m.role {
                        ChatCompletionMessageRole::System => "SYSTEM",
                        ChatCompletionMessageRole::User => "USER",
                        ChatCompletionMessageRole::Assistant => "ASSISTANT",
                        _ => "USER",
                    };
                    if let Some(content) = &m.content {
                        println!("[{}] {}", role_str, content);
                    }
                }
                continue;
            }
            "" => {
                println!("Empty message received. :(");
            }
            _ => {
                messages.push(ChatCompletionMessage {
                    role: ChatCompletionMessageRole::User,
                    content: Some(input),
                    ..Default::default()
                });
                // save user message
                let session_msgs: Vec<SessionMessage> = messages
                    .iter()
                    .map(|m| SessionMessage {
                        role: match m.role {
                            ChatCompletionMessageRole::System => "system".to_string(),
                            ChatCompletionMessageRole::User => "user".to_string(),
                            ChatCompletionMessageRole::Assistant => "assistant".to_string(),
                            _ => "user".to_string(),
                        },
                        content: m.content.clone().unwrap_or_default(),
                    })
                    .collect();
                session_manager
                    .update_session(session_id, &session_msgs)
                    .await?;
                let stream = ChatCompletionDelta::builder(model, messages.clone())
                    .credentials(credentials.clone())
                    .create_stream()
                    .await
                    .with_context(|| "Can't open Stream")?;

                let answer = ask(stream)
                    .await?
                    .choices
                    .first()
                    .with_context(|| "Can't get choices")?
                    .message
                    .clone();
                messages.push(answer);
                // save assistant response
                let session_msgs: Vec<SessionMessage> = messages
                    .iter()
                    .map(|m| SessionMessage {
                        role: match m.role {
                            ChatCompletionMessageRole::System => "system".to_string(),
                            ChatCompletionMessageRole::User => "user".to_string(),
                            ChatCompletionMessageRole::Assistant => "assistant".to_string(),
                            _ => "user".to_string(),
                        },
                        content: m.content.clone().unwrap_or_default(),
                    })
                    .collect();
                session_manager
                    .update_session(session_id, &session_msgs)
                    .await?;
            }
        }
    }
}
