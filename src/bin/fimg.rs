// src/bin/fimg.rs
//! Generate images via OpenAI Images API
use clap::Parser;
use anyhow::{bail, Context, Result};
use inquire::{Select, Confirm, Text};
use FerriteChatter::config::Config;
use openai::Credentials;
use FerriteChatter::image::{generate_images, edit_images};
use serde_json::Value;
use reqwest::{Client, multipart::{Form, Part}};
use std::fs;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Read, IsTerminal};
use std::path::{PathBuf, Path};
use std::ffi::OsStr;
use viuer::{Config as ViuerConfig, print_from_file};
use base64;

#[derive(Parser, Debug)]
#[clap(author, version, about = "Generate images with OpenAI")]
struct Args {
    /// OpenAI API Key
    #[clap(long = "key", short = 'k')]
    key: Option<String>,
    /// OpenAI API Base URL
    #[clap(long = "base-url", short = 'b')]
    base_url: Option<String>,
    /// Model to use for image generation (e.g. dall-e-2, gpt-image-1)
    #[clap(long = "model", short = 'm')]
    model: Option<String>,
    /// Path to existing image for editing
    #[clap(long = "image", short = 'i', value_parser)]
    image: Option<PathBuf>,
    /// Path to mask image for editing (PNG with transparency)
    #[clap(long = "mask", short = 'M', value_parser)]
    mask: Option<PathBuf>,
    /// Output file path (PNG). Defaults to 'fimg.png'
    #[clap(long = "output", short = 'o', value_parser)]
    output: Option<PathBuf>,
    /// Number of images to generate
    #[clap(long = "number", short = 'n', default_value = "1")]
    number: u32,
    /// Image size [1024x1024,1024x1792,1792x1024]
    #[clap(long = "size", short = 's', default_value = "1024x1024")]
    size: String,
    /// Response format [url or b64_json]
    #[clap(long = "format", short = 'f', default_value = "url")]
    response_format: String,
    /// Prompt text (omit to read from stdin)
    prompt: Option<String>,
}

#[derive(Serialize)]
struct ImageRequest {
    model: String,
    prompt: String,
    n: u32,
    size: String,
    // GPT Image models do not support response_format
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<String>,
}

#[derive(Deserialize)]
struct ImageData {
    url: Option<String>,
    b64_json: Option<String>,
}

#[derive(Deserialize)]
struct ImageResponse {
    data: Vec<ImageData>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = Config::load()?;

    let key = args.key.unwrap_or(
        config.get_openai_api_key().clone().unwrap_or(
            env::var("OPENAI_API_KEY")
                .context("API key not set via --key or OPENAI_API_KEY")?,
        ),
    );
    let base_url = args.base_url.unwrap_or(
        config.get_openai_base_url().clone().unwrap_or(
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
        ),
    );
    let credentials = Credentials::new(key, base_url);
    // Determine if editing existing image
    let editing = args.image.is_some();

    // Read prompt from stdin or CLI
    let mut stdin = io::stdin();
    let prompt = if !stdin.is_terminal() {
        let mut s = String::new();
        let _ = stdin.read_to_string(&mut s);
        s.trim_end().to_string()
    } else {
        args.prompt.clone().context("Prompt must be provided as argument or via pipe")?
    };

    let client = Client::new();
    // Determine model to use
    let model = if let Some(m) = args.model.clone() {
        m
    } else {
        // Fetch available models list
        let models_url = format!("{}/models", credentials.base_url());
        let resp = client
            .get(&models_url)
            .header("Authorization", format!("Bearer {}", credentials.api_key()))
            .send()
            .await?
            .json::<Value>()
            .await?;
        let data = resp
            .get("data")
            .and_then(|v| v.as_array())
            .context("Failed to parse models list")?;
        // Filter DALL-E models
        let mut choices: Vec<String> = data
            .iter()
            .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
            // include DALL-E and GPT Image models
            .filter(|id| id.contains("dall") || id.starts_with("gpt-image"))
            .collect();
        // ensure at least one default
        if choices.is_empty() {
            choices.push("dall-e-2".to_string());
        }
        Select::new("Select image model:", choices).prompt()?
    };
    // Prepare response_format option
    let resp_fmt = if model.starts_with("gpt-image") {
        None
    } else {
        Some(args.response_format.clone())
    };
    // Send either generation or edit request
    let resp = if args.image.is_some() {
        // Image editing
        let edit_url = format!("{}/images/edits", credentials.base_url());
        let mut form = Form::new()
            .text("model", model.clone())
            .text("prompt", prompt.clone())
            .text("n", args.number.to_string())
            .text("size", args.size.clone());
        if let Some(fmt) = resp_fmt.clone() {
            form = form.text("response_format", fmt);
        }
        // Attach image file
        // Attach image file
        let img_path = args.image.as_ref().unwrap();
        let img_bytes = fs::read(img_path)
            .with_context(|| format!("Failed to read image file {:?}", img_path))?;
        let img_part = Part::bytes(img_bytes)
            .file_name(img_path.file_name().and_then(|s| s.to_str()).unwrap_or("image.png").to_string());
        form = form.part("image", img_part);
        if let Some(mask_path) = &args.mask {
            // Attach mask file
            let mask_bytes = fs::read(mask_path)
                .with_context(|| format!("Failed to read mask file {:?}", mask_path))?;
            let mask_part = Part::bytes(mask_bytes)
                .file_name(mask_path.file_name().and_then(|s| s.to_str()).unwrap_or("mask.png").to_string());
            form = form.part("mask", mask_part);
        }
        client.post(&edit_url)
            .header("Authorization", format!("Bearer {}", credentials.api_key()))
            .multipart(form)
            .send()
            .await?
    } else {
        // Image generation
        let gen_url = format!("{}/images/generations", credentials.base_url());
        let request = ImageRequest {
            model: model.clone(),
            prompt: prompt.clone(),
            n: args.number,
            size: args.size.clone(),
            response_format: resp_fmt,
        };
        client.post(&gen_url)
            .header("Authorization", format!("Bearer {}", credentials.api_key()))
            .json(&request)
            .send()
            .await?
    };
    let status = resp.status();
    // Read response body
    let body = resp.text().await?;
    // Handle HTTP error
    if !status.is_success() {
        bail!("OpenAI API error ({})\n{}", status, body);
    }
    // Interactive editing for GPT Image models
    // Parse JSON response
    let v: Value = serde_json::from_str(&body)
        .with_context(|| format!("Invalid JSON response: {}", body))?;
    // Handle API error object
    if let Some(err) = v.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        bail!("OpenAI Images API error: {}", msg);
    }
    // Extract 'data' array
    let data = v.get("data")
        .with_context(|| format!("Missing 'data' in response: {}", body))?;
    let items: Vec<ImageData> = serde_json::from_value(data.clone())
        .with_context(|| format!("Failed to parse 'data' field: {}", data))?;
    // Save and preview images
    let cfg = ViuerConfig::default();
    let mut saved_paths: Vec<PathBuf> = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        // obtain bytes
        let bytes = if let Some(u) = &item.url {
            client.get(u).send().await?.bytes().await?.to_vec()
        } else if let Some(b64) = &item.b64_json {
            base64::decode(b64)?
        } else {
            continue;
        };
        // determine filename
        let default = PathBuf::from("fimg.png");
        let base = args.output.clone().unwrap_or(default);
        let path = if items.len() > 1 {
            let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("fimg");
            let ext = base.extension().and_then(|s| s.to_str()).unwrap_or("png");
            PathBuf::from(format!("{}_{}.{}", stem, idx+1, ext))
        } else {
            base.clone()
        };
        // write file
        fs::write(&path, &bytes)
            .with_context(|| format!("Failed to write image to {:?}", path))?;
        // display via Sixel
        let _ = print_from_file(&path, &cfg);
        println!("Saved to {:?}", path);
        saved_paths.push(path.clone());
    }
    // Interactive editing for GPT Image models
    // Interactive editing for GPT Image models (repeatable)
    if model.starts_with("gpt-image") && !saved_paths.is_empty() {
        // Use the first generated image as the base
        let mut current_path = saved_paths[0].clone();
        loop {
            let do_edit = Confirm::new("Edit generated image again?")
                .with_default(false)
                .prompt()?;
            if !do_edit {
                break;
            }
            // Ask for edit prompt
            let edit_prompt = Text::new("Edit prompt:")
                .prompt()?;
            // Call edit API
            // Call edit API, handle possible safety block
            match edit_images(
                credentials.clone(),
                &model,
                &edit_prompt,
                1,
                &args.size,
                None,
                &current_path,
                args.mask.as_deref(),
            ).await {
                Ok(mut edits) => {
                    if let Some(img) = edits.pop() {
                        // Get bytes
                        let bytes = if let Some(u) = img.url {
                            client.get(&u).send().await?.bytes().await?.to_vec()
                        } else if let Some(b64) = img.b64_json {
                            base64::decode(&b64)?
                        } else {
                            println!("No image data returned");
                            continue;
                        };
                        // Overwrite file
                        fs::write(&current_path, &bytes)
                            .with_context(|| format!("Failed to write edited image to {:?}", current_path))?;
                        // Preview
                        let _ = print_from_file(&current_path, &cfg);
                        println!("Edited image saved to {:?}", current_path);
                    } else {
                        println!("No edited image returned");
                    }
                }
                Err(err) => {
                    let msg = err.to_string();
                    if msg.contains("safety_violations") {
                        println!("編集が安全システムによって拒否されました。別のプロンプトを試してください。");
                    } else {
                        println!("Error during edit: {}", msg);
                    }
                    break;
                }
            }
        }
    }

    Ok(())
}
