use anyhow::{anyhow, Result};
use clap::ValueEnum;
use ferrite_model_gen::generate_models;
use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;
use std::convert::TryFrom;
use std::fmt;

generate_models!();
pub const DEFAULT_MODEL: Model = Model::Gpt_4o;
