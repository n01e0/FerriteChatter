use anyhow::{anyhow, Result};
use clap::ValueEnum;
use ferrite_model_gen::generate_models;
use openai::chat::{ChatCompletion, ChatCompletionDelta};
use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;
use std::convert::TryFrom;
use std::fmt;
use std::io::{stdout, Write};
use tokio::sync::mpsc::Receiver;

generate_models!();
pub const DEFAULT_MODEL: Model = Model::Gpt_4o;

pub async fn ask(mut stream: Receiver<ChatCompletionDelta>) -> Result<ChatCompletion> {
    let mut merged: Option<ChatCompletionDelta> = None;

    while let Some(delta) = stream.recv().await {
        let choice = &delta.choices[0];
        if let Some(content) = &choice.delta.content {
            print!("{content}");
        }
        if choice.finish_reason.is_some() {
            println!();
        }
        stdout().flush()?;

        match merged.as_mut() {
            Some(c) => c.merge(delta)?,
            None => merged = Some(delta),
        };
    }

    Ok(merged.unwrap().into())
}
