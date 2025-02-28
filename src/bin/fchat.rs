use anyhow::{Context, Result};
use clap::Parser;
use inquire::{Confirm, Editor, Text};
use openai::{
    chat::{ChatCompletionDelta, ChatCompletionMessage, ChatCompletionMessageRole},
    Credentials,
};
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use FerriteChatter::{
    config::Config,
    core::{ask, Model, DEFAULT_MODEL},
};

const SEED_PROMPT: &'static str = r#"
You are an engineer's assistant.
The user can reset the current state of the chat by inputting 'reset'.
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

    let mut messages = vec![ChatCompletionMessage {
        role: role,
        content: Some(args.general.unwrap_or(String::from(SEED_PROMPT))),
        ..Default::default()
    }];

    if let Some(path) = args.file {
        let mut input = String::new();
        let _ = File::open(path)?.read_to_string(&mut input);
        messages.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: Some(input),
            ..Default::default()
        })
    }

    let initial_state = messages.clone();

    loop {
        let input = Text::new("").prompt()?;
        match &input[..] {
            "exit" => {
                println!("Bye!");
                return Ok(());
            }
            "reset" => {
                messages = Vec::from(&initial_state[..]);
            }
            "v" => {
                let input = Editor::new("Prompt:").prompt()?;
                messages.push(ChatCompletionMessage {
                    role: ChatCompletionMessageRole::User,
                    content: Some(input),
                    ..Default::default()
                });
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
            }
            "save" => {
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
            "" => {
                println!("Empty message received. :(");
            }
            _ => {
                messages.push(ChatCompletionMessage {
                    role: ChatCompletionMessageRole::User,
                    content: Some(input),
                    ..Default::default()
                });
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
            }
        }
    }
}
