use anyhow::{anyhow, Context, Result};
use clap::Parser;
use openai::{
    chat::{ChatCompletionDelta, ChatCompletionMessage, ChatCompletionMessageRole},
    Credentials,
};
use std::env;
use std::fs::File;
use std::io::{self, IsTerminal, Read, Write};
use FerriteChatter::{
    config::Config,
    core::{ask, Model, DEFAULT_MODEL},
    web::{Citation, WebMessage, WebSearchClient, WebSearchResult},
};

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
    #[clap(long = "model", short = 'm', value_enum, default_value = "gpt-4o")]
    model: Option<Model>,
    /// Use Web Search API
    #[clap(long = "web")]
    web: bool,
    #[clap(long = "file", short = 'f')]
    file: Option<String>,
    /// Prompt
    prompt: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = Config::load()?;
    let mut stdin = io::stdin();
    let prompt = if !stdin.is_terminal() {
        let mut s = String::new();
        let _ = stdin.read_to_string(&mut s);
        Some(format!(
            "{}\n{}",
            s,
            args.prompt.unwrap_or(String::default())
        ))
    } else {
        args.prompt
    }
    .with_context(|| "Please provide input via a pipe or pass the prompt as an argument.")?;

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

    let web_mode = args.web;
    let model_string = if web_mode {
        args.model
            .as_ref()
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "gpt-5-search-api".to_string())
    } else {
        args.model
            .as_ref()
            .map(|m| m.as_str().to_string())
            .or_else(|| {
                config
                    .get_default_model()
                    .clone()
                    .map(|m| m.as_str().to_string())
            })
            .unwrap_or_else(|| DEFAULT_MODEL.as_str().to_string())
    };

    let requires_web_tool = if web_mode {
        !model_string.to_ascii_lowercase().contains("search-api")
    } else {
        false
    };

    let verbose_web = std::env::var("FERRITE_DEBUG_WEB")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let model = model_string.as_str();

    let role = if !model.starts_with("o1") {
        ChatCompletionMessageRole::System
    } else {
        ChatCompletionMessageRole::User
    };

    let mut messages = Vec::new();
    if let Some(general) = args.general {
        messages.push(ChatCompletionMessage {
            role,
            content: Some(general),
            ..Default::default()
        })
    }
    if let Some(path) = args.file {
        let mut input = String::new();
        let _ = File::open(path)?.read_to_string(&mut input);
        messages.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: Some(input),
            ..Default::default()
        })
    }

    messages.push(ChatCompletionMessage {
        role: ChatCompletionMessageRole::User,
        content: Some(prompt),
        ..Default::default()
    });

    if web_mode {
        let web_client = WebSearchClient::new();
        let web_messages: Vec<WebMessage> = messages
            .iter()
            .filter_map(|m| {
                m.content.as_ref().map(|content| WebMessage {
                    role: match m.role {
                        ChatCompletionMessageRole::System => "system".to_string(),
                        ChatCompletionMessageRole::Assistant => "assistant".to_string(),
                        _ => "user".to_string(),
                    },
                    content: content.clone(),
                })
            })
            .collect();

        let WebSearchResult {
            message,
            citations,
            displayed,
        } = web_client
            .stream_response(
                &credentials,
                model,
                &web_messages,
                requires_web_tool,
                |delta| {
                    print!("{delta}");
                    io::stdout().flush().map_err(|e| anyhow!(e))
                },
                verbose_web,
            )
            .await?;

        if !displayed && !message.is_empty() {
            println!("{message}");
        }
        if !citations.is_empty() {
            println!();
            println!("--- Sources ---");
            for (idx, Citation { url, title }) in citations.iter().enumerate() {
                if let Some(title) = title {
                    println!("[{}] {} ({})", idx + 1, title, url);
                } else {
                    println!("[{}] {}", idx + 1, url);
                }
            }
        }
        Ok(())
    } else {
        let stream = ChatCompletionDelta::builder(model, messages.clone())
            .credentials(credentials.clone())
            .create_stream()
            .await
            .with_context(|| "Can't open Stream")?;

        ask(stream).await.map(|_| ())
    }
}
