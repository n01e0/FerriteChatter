extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::env;
use anyhow::Context;

#[derive(Deserialize)]
struct Model {
    id: String,
}

#[proc_macro]
pub fn generate_models(_input: TokenStream) -> TokenStream {
    // OpenAI APIのエンドポイント
    let api_url = "https://api.openai.com/v1/models";

    // OpenAI APIキーを環境変数から取得
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    // HTTPクライアントを作成してAPIを呼び出し、モデルのリストを取得
    let client = Client::new();
    let res = client
        .get(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .expect("Failed to send request");

    let models: Vec<Model> = res
        .json::<serde_json::Value>()
        .expect("Failed to parse response")
        .get("data")
        .expect("Missing 'data' field in response")
        .as_array()
        .expect("'data' is not an array")
        .iter()
        .map(|model| serde_json::from_value(model.clone()).expect("Failed to deserialize model"))
        .filter(|m: &Model| m.id.contains("gpt") || m.id.contains("o1") || m.id.contains("o3") || m.id.contains("o4"))
        .filter(|m| !m.id.contains("audio") && !m.id.contains("realtime")) // remove audio models
        .filter(|m| !m.id.contains("gpt-5") || m.id == "gpt-5-chat-latest") // gpt-5-chat
        .filter(|m| !m.id.starts_with("gpt-image-")) // ignore gpt-image-*
        .collect();

    // 各モデルに対応するenumとimplを生成
    let enum_variants: Vec<_> = models
        .iter()
        .map(|m| {
            let variant_name = to_camel_case(&m.id);
            let variant_str = &m.id;
            quote! {
                #[clap(name = #variant_str)]
                #variant_name,
            }
        })
        .collect();

    let match_arms: Vec<_> = models
        .iter()
        .map(|m| {
            let variant_name = to_camel_case(&m.id);
            let variant_str = &m.id;
            quote! {
                #variant_str => Ok(Model::#variant_name),
            }
        })
        .collect();

    let as_str_arms: Vec<_> = models
        .iter()
        .map(|m| {
            let variant_name = to_camel_case(&m.id);
            let variant_str = &m.id;
            quote! {
                Model::#variant_name => #variant_str,
            }
        })
        .collect();

    let expanded = quote! {
        #[derive(Debug, Eq, PartialEq, ValueEnum, Clone)]
        #[allow(non_camel_case_types)]
        pub enum Model {
            #(#enum_variants)*
        }

        impl TryFrom<&str> for Model {
            type Error = anyhow::Error;
            fn try_from(value: &str) -> Result<Model, Self::Error> {
                match value {
                    #(#match_arms)*
                    _ => Err(anyhow!("Unknown Model. If a model does not exist to support it, please create an issue at github.com/n
01e0/FerriteChatter/issues/new.")),
                }
            }
        }

        impl Model {
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(#as_str_arms)*
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
    };

    TokenStream::from(expanded)
}

// Helper function to convert snake_case to CamelCase
fn to_camel_case(s: &str) -> syn::Ident {
    // .を_に置き換え、-を無視して文字列をキャメルケースに変換する
    let replaced: String = s.chars()
        .map(|c| {
            if c == '.' {
                '_'
            } else {
                c
            }
        })
        .collect();

    let camel_case = replaced
        .split('-')
        .map(|word| capitalize(word))
        .collect::<Vec<String>>()
        .join("_");

    syn::parse_str(&camel_case).with_context(|| format!("While parsing {}", s)).unwrap()
}


// Helper function to capitalize the first letter of a word
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
