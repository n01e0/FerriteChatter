use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use inquire::{Text, Editor};
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
    /// Model. now only "gpt-3.5-turbo" and "gpt-3.5-turbo-0301" supported.
    /// default is "gpt-3.5-turbo"
    #[clap(long = "model", short = 'm', value_enum, default_value = "gpt-3.5-turbo")]
    model: Option<Model>,
}

#[derive(Debug, Eq, PartialEq, ValueEnum, Clone)]
#[allow(non_camel_case_types)]
enum Model {
    //    Gpt_4,
    //    Gpt_4_0314,
    //    Gpt_4_32k,
    //    Gpt_4_32k_0314,
    #[clap(name = "gpt-3.5-turbo")]
    Gpt_3_5_Turbo,
    #[clap(name = "gpt-3.5-turbo-0301")]
    Gpt_3_5_Turbo_0301,
}

impl Model {
    fn as_str(&self) -> &'static str {
        match self {
            //            Self::Gpt_4 => "gpt-4",
            //            Self::Gpt_4_0314 => "gpt-4-0314",
            //            Self::Gpt_4_32k => "gpt-4-32k",
            //            Self::Gpt_4_32k_0314 => "gpt-4-32k-0314",
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
                .unwrap_or(String::from("You are an engineer's assistant.")),
            name: None,
        },
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: String::from(
                "The user can reset the current state of the chat by inputting 'reset'.",
            ),
            name: None,
        },
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: String::from("To terminate, the user needs to input \"exit\"."),
            name: None,
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
                println!("{:?}: {}", &answer.role, &answer.content.trim());
                messages.push(answer);
            }
            _ => {
                let answer = ask(&mut messages, input, model).await?;
                println!("{:?}: {}", &answer.role, &answer.content.trim());
                messages.push(answer);
            }
        }
    }
}

async fn ask(messages: &mut Vec<ChatCompletionMessage>, input: String, model: &str) -> Result<ChatCompletionMessage> {
    messages.push(ChatCompletionMessage {
        role: ChatCompletionMessageRole::User,
        content: String::from(input),
        name: None,
    });

    let chat_completion = ChatCompletion::builder(model, messages.clone())
        .create()
        .await??;
    let answer = chat_completion
        .choices
        .first()
        .with_context(|| "Can't read ChatGPT output")?
        .message
        .clone();
    return Ok(answer)
}
