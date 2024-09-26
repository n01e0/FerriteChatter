use anyhow::{Context, Result};
use clap::Parser;
use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    set_key,
};
use std::env;
use std::io::{self, IsTerminal, Read};
use FerriteChatter::{
    config::Config,
    core::{Model, DEFAULT_MODEL},
};

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
    prompt: Option<String>,
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
    set_key(key);

    let mut messages = vec![ChatCompletionMessage {
        role: ChatCompletionMessageRole::System,
        content: Some(args.general.unwrap_or(String::from(
            "次の文章を、日本語の場合は英語に、日本語以外の場合は日本語に翻訳してください。",
        ))),
        name: None,
        function_call: None,
    }];

    let model = args
        .model
        .unwrap_or(config.get_default_model().clone().unwrap_or(DEFAULT_MODEL))
        .as_str();

    let mut stdin = io::stdin();
    let prompt = args.prompt.unwrap_or(
        if !stdin.is_terminal() {
            let mut s = String::new();
            let _ = stdin.read_to_string(&mut s);
            Some(s)
        } else {
            None
        }
        .with_context(|| "Please provide input via a pipe or pass the prompt as an argument.")?,
    );

    messages.push(ChatCompletionMessage {
        role: ChatCompletionMessageRole::User,
        content: Some(prompt),
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
