use crate::core;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::fs::{create_dir_all, read_to_string};
use std::path::Path;
use tia::Tia;

#[derive(Debug, Tia, Deserialize)]
#[tia(rg)]
pub struct Config {
    openai_api_key: Option<String>,
    openai_base_url: Option<String>,
    default_model: Option<core::Model>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            openai_base_url: None,
            default_model: Some(crate::core::Model::Gpt_4o),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // Determine configuration directory: $XDG_CONFIG_HOME/ferrite or $HOME/.config/ferrite
        let home = env::var("HOME").with_context(|| "Where is the HOME?")?;
        let base = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{home}/.config"));
        let conf_dir = Path::new(&base).join("ferrite");
        create_dir_all(&conf_dir)
            .with_context(|| format!("Can't create config directory {:?}", &conf_dir))?;
        let config_path = conf_dir.join("ferriteconf.yaml");

        if !config_path.exists() {
            Ok(Self::default())
        } else {
            let content = read_to_string(&config_path)
                .with_context(|| format!("Can't read config file {:?}", &config_path))?;
            serde_yaml::from_str(&content)
                .with_context(|| format!("Can't parse config file {:?}", &config_path))
        }
    }
}
