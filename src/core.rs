use clap::ValueEnum;

#[derive(Debug, Eq, PartialEq, ValueEnum, Clone)]
#[allow(non_camel_case_types)]
pub enum Model {
    #[clap(name = "gpt-4")]
    Gpt_4,
    #[clap(name = "gpt-4-0314")]
    Gpt_4_0314,
    #[clap(name = "gpt-4-0613")]
    Gpt_4_0613,
    #[clap(name = "gpt-4-32k")]
    Gpt_4_32k,
    #[clap(name = "gpt-4-32k-0613")]
    Gpt_4_32k_0314,
    #[clap(name = "gpt-4-1106-preview")]
    Gpt_4_1106_Preview,
    #[clap(name = "gpt-3.5-turbo")]
    Gpt_3_5_Turbo,
    #[clap(name = "gpt-3.5-turbo-16k")]
    Gpt_3_5_Turbo_16k,
    #[clap(name = "gpt-3.5-turbo-0301")]
    Gpt_3_5_Turbo_0301,
    #[clap(name = "gpt-3.5-turbo-0613")]
    Gpt_3_5_Turbo_0613,
    #[clap(name = "gpt-3.5-turbo-0613")]
    Gpt_3_5_Turbo_1106,
    #[clap(name = "gpt-3.5-turbo-16k-0613")]
    Gpt_3_5_Turbo_16k_0613,
}

impl Model {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gpt_4 => "gpt-4",
            Self::Gpt_4_0314 => "gpt-4-0314",
            Self::Gpt_4_0613 => "gpt-4-0613",
            Self::Gpt_4_32k => "gpt-4-32k",
            Self::Gpt_4_32k_0314 => "gpt-4-32k-0613",
            Self::Gpt_4_1106_Preview => "gpt-4-1106-preview",
            Self::Gpt_3_5_Turbo_16k => "gpt-3.5-turbo-16k",
            Self::Gpt_3_5_Turbo => "gpt-3.5-turbo",
            Self::Gpt_3_5_Turbo_0301 => "gpt-3.5-turbo-0301",
            Self::Gpt_3_5_Turbo_0613 => "gpt-3.5-turbo-0613",
            Self::Gpt_3_5_Turbo_1106 => "gpt-3.5-turbo-1106",
            Self::Gpt_3_5_Turbo_16k_0613 => "gpt-3.5-turbo-16k-0613",
        }
    }
}
