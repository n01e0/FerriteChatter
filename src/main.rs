use anyhow::{Context, Result};
use clap::Parser;
use inquire::Text;
use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    set_key,
};
use std::env;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Open Prompt(General Prompt)
    #[clap(long = "general", short = 'g')]
    general: Option<String>,
    /// OenAI API Key
    #[clap(long = "key", short = 'k')]
    key: Option<String>,
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
                .unwrap_or(String::from("You are an engineer's assistant.")),
            name: None,
        },
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: String::from(
                "To terminate, the user needs to input \"exit\"."
            ),
            name: None,
        },
    ];

    loop {
        let input = Text::new("").prompt()?;
        if &input == "exit" {
            println!("Bye!");
            return Ok(());
        }
        messages.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: input,
            name: None,
        });

        let chat_completion = ChatCompletion::builder("gpt-3.5-turbo", messages.clone())
            .create()
            .await??;
        let answer = chat_completion
            .choices
            .first()
            .with_context(|| "Can't read ChatGPT output")?
            .message
            .clone();
        println!("{:?}: {}", &answer.role, &answer.content.trim());
        messages.push(answer);
    }
}
