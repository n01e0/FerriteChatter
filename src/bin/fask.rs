use anyhow::{Context, Result};
use clap::Parser;
use openai_fork::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    set_key,
};
use std::env;
use FerriteChatter::core::Model;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Open Prompt(General Prompt)
    #[clap(long = "general", short = 'g')]
    general: Option<String>,
    /// OenAI API Key
    #[clap(long = "key", short = 'k')]
    key: Option<String>,
    /// OpenAI Model
    #[clap(long = "model", short = 'm', value_enum, default_value = "gpt-4o")]
    model: Option<Model>,
    /// Prompt
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let key = args.key.unwrap_or(
        env::var("OPENAI_API_KEY")
            .with_context(|| "You need to set API key to the `OPENAI_API_KEY`")?,
    );
    set_key(key);

    let mut messages = Vec::new();
    if let Some(general) = args.general {
        messages.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: Some(general),
            name: None,
            function_call: None,
        })
    }

    let model = args
        .model
        .map(|m| m.as_str())
        .with_context(|| "something wrong")?;

    messages.push(ChatCompletionMessage {
        role: ChatCompletionMessageRole::User,
        content: Some(args.prompt),
        name: None,
        function_call: None,
    });

    let chat_completion = ChatCompletion::builder(model, messages.clone())
        .create()
        .await?;
    let answer = &chat_completion
        .choices
        .first()
        .with_context(|| "Can't read ChatGPT output")?
        .message;

    println!(
        "{}",
        answer
            .content
            .clone()
            .with_context(|| "Can't get content")?
            .trim()
    );
    Ok(())
}
