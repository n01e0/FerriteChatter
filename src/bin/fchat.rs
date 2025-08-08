use anyhow::{Context, Result};
use base64;
use clap::Parser;
use inquire::{Confirm, Editor, Select, Text};
use openai::{
    chat::{ChatCompletion, ChatCompletionDelta, ChatCompletionMessage, ChatCompletionMessageRole},
    Credentials,
};
use reqwest::Client;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{env, fs};
use viuer::{print_from_file, Config as ViuerConfig};
use FerriteChatter::image::{edit_images, generate_images, ImageData};
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
    // System prompt for Japanese summary (skip the first session system message below)
    messages.push(ChatCompletionMessage {
        role: ChatCompletionMessageRole::System,
        content: Some("次の会話内容を一文で簡潔に日本語で要約してください：".to_string()),
        ..Default::default()
    });
    // Include only user and assistant messages, skip session's first System prompt
    for (i, m) in session_msgs.iter().enumerate() {
        // skip the initial system prompt stored in session
        if i == 0 && m.role == "system" {
            continue;
        }
        // include only user and assistant roles
        if m.role != "user" && m.role != "assistant" {
            continue;
        }
        let role = if m.role == "assistant" {
            ChatCompletionMessageRole::Assistant
        } else {
            ChatCompletionMessageRole::User
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

fn session_scorer(input: &str, option: &String, string_value: &str, index: usize) -> Option<i64> {
    if option == "New session" {
        Some(i64::MAX)
    } else {
        Select::<String>::DEFAULT_SCORER(input, option, string_value, index)
    }
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
    let session_manager = SessionManager::new()?;

    let mut session_id: Option<i64> = None;
    let mut messages: Vec<ChatCompletionMessage> = Vec::new();
    // Prepare new session messages: system prompt and optional file content
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
    let mut initial_state = messages.clone();
    // HTTP client for image retrieval
    let client_http = Client::new();
    // Last generated image path for editing
    let mut last_image_path: Option<PathBuf> = None;

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
                // save user message (create session if needed)
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
                if let Some(id) = session_id {
                    session_manager.update_session(id, &session_msgs)?;
                } else {
                    let id = session_manager.create_session("", &session_msgs)?;
                    session_id = Some(id);
                }
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
                session_manager.update_session(session_id.unwrap(), &session_msgs)?;
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
                // List sessions in descending order (newest first)
                let mut existing_sessions = session_manager.list_sessions()?;
                existing_sessions.sort_by(|a, b| b.0.cmp(&a.0));
                if existing_sessions.is_empty() {
                    println!("No sessions available.");
                } else {
                    // Inline preview labels for session switching
                    let mut labels = Vec::new();
                    let mut ids = Vec::new();
                    for (id, _name, summary_opt) in &existing_sessions {
                        // Skip sessions that contain only system messages
                        let msgs = session_manager.load_session(*id)?;
                        if msgs.iter().all(|m| m.role == "system") {
                            continue;
                        }
                        // Use or generate summary preview
                        let summary = if let Some(s) = summary_opt {
                            s.clone()
                        } else {
                            let msgs = session_manager.load_session(*id)?;
                            let s = generate_summary(&msgs, credentials.clone(), model).await?;
                            session_manager.update_summary(*id, &s)?;
                            s
                        };
                        // Use summary as the selection label
                        labels.push(summary.clone());
                        ids.push(*id);
                    }
                    let selection = Select::new("Choose a session:", labels.clone()).prompt()?;
                    // Find selected id and load messages
                    if let Some(idx) = labels.iter().position(|l| l == &selection) {
                        let sel_id = ids[idx];
                        let loaded = session_manager.load_session(sel_id)?;
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
                        session_id = Some(sel_id);
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
            cmd if cmd.starts_with("/img ") => {
                // Image generation
                let prompt_img = cmd.trim_start_matches("/img").trim();
                match generate_images(
                    credentials.clone(),
                    "dall-e-2",
                    prompt_img,
                    1,
                    "1024x1024",
                    Some("url"),
                )
                .await
                {
                    Ok(images) => {
                        let cfg = ViuerConfig::default();
                        for img in images {
                            if let Some(url) = img.url {
                                if let Ok(resp) = client_http.get(&url).send().await {
                                    if let Ok(bytes) = resp.bytes().await {
                                        // save to temp file
                                        let tmp = env::temp_dir().join("fchat_image.png");
                                        let _ = fs::write(&tmp, &bytes);
                                        // display via Sixel
                                        let _ = print_from_file(&tmp, &cfg);
                                        last_image_path = Some(tmp);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => println!("Image generation error: {}", e),
                }
                continue;
            }
            cmd if cmd.starts_with("/edit ") => {
                // Image editing
                let prompt_edit = cmd.trim_start_matches("/edit").trim();
                if let Some(ref img_path) = last_image_path {
                    match edit_images(
                        credentials.clone(),
                        "gpt-image-1",
                        prompt_edit,
                        1,
                        "1024x1024",
                        None,
                        img_path,
                        None,
                    )
                    .await
                    {
                        Ok(images) => {
                            let cfg = ViuerConfig::default();
                            for img in images {
                                if let Some(url) = img.url {
                                    if let Ok(resp) = client_http.get(&url).send().await {
                                        if let Ok(bytes) = resp.bytes().await {
                                            // save to temp file
                                            let tmp = env::temp_dir().join("fchat_image.png");
                                            let _ = fs::write(&tmp, &bytes);
                                            // display via Sixel
                                            let _ = print_from_file(&tmp, &cfg);
                                            last_image_path = Some(tmp);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => println!("Image edit error: {}", e),
                    }
                } else {
                    println!("No image available for editing. Use /img first.");
                }
                continue;
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
                if let Some(id) = session_id {
                    session_manager.update_session(id, &session_msgs)?;
                } else {
                    let id = session_manager.create_session("", &session_msgs)?;
                    session_id = Some(id);
                }
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
                session_manager.update_session(session_id.unwrap(), &session_msgs)?;
            }
        }
    }
}
