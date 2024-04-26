use anyhow::{Context, Result};
use clap::Parser;
use inquire::{Editor, Text};
use openai::{
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
    #[clap(
        long = "model",
        short = 'm',
        value_enum,
        default_value = "gpt-4-1106-preview"
    )]
    model: Option<Model>,
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
            content: Some(args
                .general
                .unwrap_or(String::from("You are an engineer's assistant."))),
            name: None,
            function_call: None,
        },
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: Some(String::from(
                "The user can reset the current state of the chat by inputting 'reset'.",
            )),
            name: None,
            function_call: None,
        },
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: Some(String::from(
                    "The user can activate the editor by entering 'v', allowing them to input multiple lines of prompts."
                )),
            name: None,
            function_call: None,
        },
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: Some(String::from("To terminate, the user needs to input \"exit\".")),
            name: None,
            function_call: None,
        },
    ];

    let initial_state = messages.clone();

    let model = args.model.unwrap().as_str();

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
                let answer = ask(&mut messages, input, model).await?;
                let content = answer
                    .content
                    .as_ref()
                    .with_context(|| "Can't get content")?;
                println!("{:?}: {}", &answer.role, content.trim());
                messages.push(answer);
            }
            _ => {
                let answer = ask(&mut messages, input, model).await?;
                let content = answer
                    .content
                    .as_ref()
                    .with_context(|| "Can't get content")?;
                println!("{:?}: {}", &answer.role, content.trim());
                messages.push(answer);
            }
        }
    }
}

async fn ask(
    messages: &mut Vec<ChatCompletionMessage>,
    input: String,
    model: &str,
) -> Result<ChatCompletionMessage> {
    messages.push(ChatCompletionMessage {
        role: ChatCompletionMessageRole::User,
        content: Some(input),
        name: None,
        function_call: None,
    });

    let chat_completion = ChatCompletion::builder(model, messages.clone())
        .create()
        .await?;
    let answer = chat_completion
        .choices
        .first()
        .with_context(|| "Can't read ChatGPT output")?
        .message
        .clone();
    Ok(answer)
}
