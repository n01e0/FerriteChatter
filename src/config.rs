use crate::core;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::fs::read_to_string;
use std::path::Path;
use tia::Tia;

#[derive(Debug, Tia, Deserialize)]
#[tia(rg)]
pub struct Config {
    openai_api_key: Option<String>,
    default_model: Option<core::Model>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            default_model: Some(crate::core::Model::Gpt_4o),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = format!(
            "{}/.ferriteconf",
            env::var("XDG_CONFIG_HOME").unwrap_or(format!(
                "{}/.config",
                env::var("HOME").with_context(|| "Where is the HOME?")?
            ))
        );

        if !Path::new(&path).exists() {
            Ok(Self::default())
        } else {
            serde_yaml::from_str(&read_to_string(path).with_context(|| "Can't read config file")?)
                .with_context(|| "Can't parse config file")
        }
    }
}
