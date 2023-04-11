use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    set_key,
};
use std::env;
use inquire::Text;
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    set_key(env::var("OPENAI_API_KEY").with_context(|| "You need to set API key to the `OPENAI_API_KEY`")?);

    let mut messages = vec!{ChatCompletionMessage {
        role: ChatCompletionMessageRole::System,
        content: String::from(
            "You are an engineer's assistant."
        ),
        name: None,
    }};

    loop {
        let input = Text::new("> ").prompt()?;
        if &input == "exit" {
            println!("Bye!");
            return Ok(())
        }
        messages.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: input,
            name: None,
        });

        let chat_completion = ChatCompletion::builder("gpt-3.5-turbo", messages.clone())
            .create()
            .await??;
        let answer = chat_completion.choices.first().with_context(|| "Can't read ChatGPT output")?.message.clone();
        println!("{:?}: {}", &answer.role, &answer.content.trim());
        messages.push(answer);
    }

}
