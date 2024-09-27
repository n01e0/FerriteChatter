use anyhow::{anyhow, Result};
use clap::ValueEnum;
use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;
use std::convert::TryFrom;
use std::fmt;
use model_gen::generate_models;


generate_models!();
pub const DEFAULT_MODEL: Model = Model::Gpt_4o;
