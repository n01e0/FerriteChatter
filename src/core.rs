use anyhow::{anyhow, Result};
use clap::ValueEnum;
use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;
use std::convert::TryFrom;
use std::fmt;

#[derive(Debug, Eq, PartialEq, ValueEnum, Clone)]
#[allow(non_camel_case_types)]
pub enum Model {
    #[clap(name = "chatgpt-4o-latest")]
    ChatGPT_4o_latest,
    #[clap(name = "gpt-4")]
    Gpt_4,
    #[clap(name = "gpt-4o")]
    Gpt_4o,
    #[clap(name = "gpt-4o-2024-05-13")]
    Gpt_4o_2024_05_13,
    #[clap(name = "gpt-4o-2024-08-06")]
    Gpt_4o_2024_08_06,
    #[clap(name = "gpt-4o-mini")]
    Gpt_4o_mini,
    #[clap(name = "gpt-4o-mini-2024-07-18")]
    Gpt_4o_mini_2024_07_18,
    #[clap(name = "gpt-4-0314")]
    Gpt_4_0314,
    #[clap(name = "gpt-4-0613")]
    Gpt_4_0613,
    #[clap(name = "gpt-4-32k")]
    Gpt_4_32k,
    #[clap(name = "gpt-4-32k-0613")]
    Gpt_4_32k_0314,
    #[clap(name = "gpt-4-0125-preview")]
    Gpt_4_0125_Preview,
    #[clap(name = "gpt-4-1106-preview")]
    Gpt_4_1106_Preview,
    #[clap(name = "gpt-4-turbo")]
    Gpt_4_Turbo,
    #[clap(name = "gpt-4-turbo-2024-04-09")]
    Gpt_4_Turbo_2024_04_09,
    #[clap(name = "gpt-4-turbo-preview")]
    Gpt_4_Turbo_Preview,
    #[clap(name = "gpt-3.5-turbo")]
    Gpt_3_5_Turbo,
    #[clap(name = "gpt-3.5-turbo-0125")]
    Gpt_3_5_Turbo_0125,
    #[clap(name = "gpt-3.5-turbo-0301")]
    Gpt_3_5_Turbo_0301,
    #[clap(name = "gpt-3.5-turbo-0613")]
    Gpt_3_5_Turbo_0613,
    #[clap(name = "gpt-3.5-turbo-0613")]
    Gpt_3_5_Turbo_1106,
    #[clap(name = "gpt-3.5-turbo-16k")]
    Gpt_3_5_Turbo_16k,
    #[clap(name = "gpt-3.5-turbo-16k-0613")]
    Gpt_3_5_Turbo_16k_0613,
}

pub const DEFAULT_MODEL: Model = Model::Gpt_4o;

impl TryFrom<&str> for Model {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Model, Self::Error> {
        match value {
            "chatgpt-4o-latest" => Ok(Model::ChatGPT_4o_latest),
            "gpt-4" => Ok(Model::Gpt_4),
            "gpt-4o" => Ok(Model::Gpt_4o),
            "gpt-4o-2024-05-13" => Ok(Model::Gpt_4o_2024_05_13),
            "gpt-4o-2024-08-06" => Ok(Model::Gpt_4o_2024_08_06),
            "gpt-4o-mini" => Ok(Model::Gpt_4o_mini),
            "gpt-4o-mini-2024-07-18" => Ok(Model::Gpt_4o_mini_2024_07_18),
            "gpt-4-0314" => Ok(Model::Gpt_4_0314),
            "gpt-4-0613" => Ok(Model::Gpt_4_0613),
            "gpt-4-32k" => Ok(Model::Gpt_4_32k),
            "gpt-4-32k-0613" => Ok(Model::Gpt_4_32k_0314),
            "gpt-4-0125-preview" => Ok(Model::Gpt_4_0125_Preview),
            "gpt-4-1106-preview" => Ok(Model::Gpt_4_1106_Preview),
            "gpt-4-turbo" => Ok(Model::Gpt_4_Turbo),
            "gpt-4-turbo-2024-04-09" => Ok(Model::Gpt_4_Turbo_2024_04_09),
            "gpt-4-turbo-preview" => Ok(Model::Gpt_4_Turbo_Preview),
            "gpt-3.5-turbo" => Ok(Model::Gpt_3_5_Turbo),
            "gpt-3.5-turbo-0125" => Ok(Model::Gpt_3_5_Turbo_0125),
            "gpt-3.5-turbo-0301" => Ok(Model::Gpt_3_5_Turbo_0301),
            "gpt-3.5-turbo-0613" => Ok(Model::Gpt_3_5_Turbo_0613),
            "gpt-3.5-turbo-1106" => Ok(Model::Gpt_3_5_Turbo_1106),
            "gpt-3.5-turbo-16k" => Ok(Model::Gpt_3_5_Turbo_16k),
            "gpt-3.5-turbo-16k-0613" => Ok(Model::Gpt_3_5_Turbo_16k_0613),
            _ => Err(anyhow!("Unknown Model. If a model does not exist to support it, please create an issue at github.com/n01e0/FerriteChatter/issues/new.")),
        }
    }
}


impl Model {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChatGPT_4o_latest => "chatgpt-4o-latest",
            Self::Gpt_4 => "gpt-4",
            Self::Gpt_4o => "gpt-4o",
            Self::Gpt_4o_2024_05_13 => "gpt-4o-2024-05-13",
            Self::Gpt_4o_2024_08_06 => "gpt-4o-2024-08-06",
            Self::Gpt_4o_mini => "gpt-4o-mini",
            Self::Gpt_4o_mini_2024_07_18 => "gpt-4o-mini-2024-07-18",
            Self::Gpt_4_0314 => "gpt-4-0314",
            Self::Gpt_4_0613 => "gpt-4-0613",
            Self::Gpt_4_32k => "gpt-4-32k",
            Self::Gpt_4_32k_0314 => "gpt-4-32k-0613",
            Self::Gpt_4_0125_Preview => "gpt-4-0125-preview",
            Self::Gpt_4_1106_Preview => "gpt-4-1106-preview",
            Self::Gpt_4_Turbo => "gpt-4-turbo",
            Self::Gpt_4_Turbo_2024_04_09 => "gpt-4-turbo-2024-04-09",
            Self::Gpt_4_Turbo_Preview => "gpt-4-turbo-preview",
            Self::Gpt_3_5_Turbo => "gpt-3.5-turbo",
            Self::Gpt_3_5_Turbo_0125 => "gpt-3.5-turbo-0125",
            Self::Gpt_3_5_Turbo_0301 => "gpt-3.5-turbo-0301",
            Self::Gpt_3_5_Turbo_0613 => "gpt-3.5-turbo-0613",
            Self::Gpt_3_5_Turbo_1106 => "gpt-3.5-turbo-1106",
            Self::Gpt_3_5_Turbo_16k => "gpt-3.5-turbo-16k",
            Self::Gpt_3_5_Turbo_16k_0613 => "gpt-3.5-turbo-16k-0613",
        }
    }
}


impl<'de> Deserialize<'de> for Model {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ModelVisitor;

        impl<'de> Visitor<'de> for ModelVisitor {
            type Value = Model;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing a model")
            }

            fn visit_str<E>(self, value: &str) -> Result<Model, E>
            where
                E: de::Error,
            {
                Model::try_from(value).map_err(|e| de::Error::custom(e.to_string()))
            }
        }
        deserializer.deserialize_str(ModelVisitor)
    }
}
