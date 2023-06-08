use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    set_key,
};
use std::env;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Open Prompt(General Prompt)
    #[clap(long = "general", short = 'g')]
    general: Option<String>,
    /// OenAI API Key
    #[clap(long = "key", short = 'k')]
    key: Option<String>,
    /// default is "gpt-4-32k"
    #[clap(long = "model", short = 'm', value_enum, default_value = "gpt-4")]
    model: Option<Model>,
    /// Prompt
    prompt: String,
}

#[derive(Debug, Eq, PartialEq, ValueEnum, Clone)]
#[allow(non_camel_case_types)]
enum Model {
    #[clap(name = "gpt-4")]
    Gpt_4,
    #[clap(name = "gpt-4-0314")]
    Gpt_4_0314,
    #[clap(name = "gpt-4-32k")]
    Gpt_4_32k,
    #[clap(name = "gpt-4-32k-0314")]
    Gpt_4_32k_0314,
    #[clap(name = "gpt-3.5-turbo")]
    Gpt_3_5_Turbo,
    #[clap(name = "gpt-3.5-turbo-0301")]
    Gpt_3_5_Turbo_0301,
}

impl Model {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Gpt_4 => "gpt-4",
            Self::Gpt_4_0314 => "gpt-4-0314",
            Self::Gpt_4_32k => "gpt-4-32k",
            Self::Gpt_4_32k_0314 => "gpt-4-32k-0314",
            Self::Gpt_3_5_Turbo => "gpt-3.5-turbo",
            Self::Gpt_3_5_Turbo_0301 => "gpt-3.5-turbo-0301",
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let key = args.key.unwrap_or(
        env::var("OPENAI_API_KEY")
            .with_context(|| "You need to set API key to the `OPENAI_API_KEY`")?,
    );
    set_key(key);

    let mut messages = vec![
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: args
                .general
                .unwrap_or(String::from("次の文章を、日本語の場合は英語に、日本語以外の場合は日本語に翻訳してください。")),
            name: None,
        },
    ];

    let model = args.model.map(|m| m.as_str()).with_context(|| "something wrong")?;

    messages.push(ChatCompletionMessage {
        role: ChatCompletionMessageRole::User,
        content: args.prompt,
        name: None,
    });

    let chat_completion = ChatCompletion::builder(model, messages.clone())
        .create()
        .await?;
    let answer = &chat_completion
        .choices
        .first()
        .with_context(|| "Can't read ChatGPT output")?
        .message;
    println!("{}", answer.content.trim());
    Ok(())
}
