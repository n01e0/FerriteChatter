use anyhow::{Context, Result};
use clap::Parser;
use inquire::{Confirm, Editor, Select, Text};
use openai::{
    chat::{ChatCompletionDelta, ChatCompletionMessage, ChatCompletionMessageRole},
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
        let mut names: Vec<String> = existing_sessions.iter().map(|(_, n)| n.clone()).collect();
        names.push("New session".to_string());
        let selection = Select::new("Choose a session:", names).prompt()?;
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
            session_id = session_manager.create_session(&name, &session_msgs).await?;
        } else {
            let (id, _) = existing_sessions
                .iter()
                .find(|(_, n)| *n == selection)
                .unwrap();
            session_id = *id;
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
                    let names: Vec<String> =
                        existing_sessions.iter().map(|(_, n)| n.clone()).collect();
                    let selection = Select::new("Choose a session:", names).prompt()?;
                    let (new_id, _) = existing_sessions
                        .iter()
                        .find(|(_, n)| *n == selection)
                        .unwrap();
                    session_id = *new_id;
                    let loaded = session_manager.load_session(session_id).await?;
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
                    println!("Switched to session: {}", selection);
                    initial_state = messages.clone();
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
