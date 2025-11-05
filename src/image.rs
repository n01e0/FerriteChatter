use anyhow::{bail, Context, Result};
use openai::Credentials;
use reqwest::{
    multipart::{Form, Part},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::Path};

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    n: u32,
    size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ImageData {
    pub url: Option<String>,
    pub b64_json: Option<String>,
}

/// Generate new images
pub async fn generate_images(
    credentials: Credentials,
    model: &str,
    prompt: &str,
    n: u32,
    size: &str,
    response_format: Option<&str>,
) -> Result<Vec<ImageData>> {
    let client = Client::new();
    let url = format!("{}/images/generations", credentials.base_url());
    let req = GenerateRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
        n,
        size: size.to_string(),
        response_format: response_format.map(|s| s.to_string()),
    };
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", credentials.api_key()))
        .json(&req)
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        bail!("OpenAI API error ({})\n{}", status, body);
    }
    let v: Value =
        serde_json::from_str(&body).with_context(|| format!("Invalid JSON response:\n{body}"))?;
    if let Some(err) = v.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        bail!("OpenAI Images API error: {}", msg);
    }
    let data = v
        .get("data")
        .with_context(|| format!("Missing 'data':\n{body}"))?;
    let items: Vec<ImageData> = serde_json::from_value(data.clone())
        .with_context(|| format!("Failed to parse 'data':\n{data:?}"))?;
    Ok(items)
}

/// Edit existing images (GPT Image models)
#[allow(clippy::too_many_arguments)]
pub async fn edit_images(
    credentials: Credentials,
    model: &str,
    prompt: &str,
    n: u32,
    size: &str,
    response_format: Option<&str>,
    image_path: &Path,
    mask_path: Option<&Path>,
) -> Result<Vec<ImageData>> {
    let client = Client::new();
    let url = format!("{}/images/edits", credentials.base_url());
    let mut form = Form::new()
        .text("model", model.to_string())
        .text("prompt", prompt.to_string())
        .text("n", n.to_string())
        .text("size", size.to_string());
    if let Some(fmt) = response_format {
        form = form.text("response_format", fmt.to_string());
    }
    // image file
    let img_bytes = fs::read(image_path)
        .with_context(|| format!("Failed to read image file {image_path:?}"))?;
    let img_name = image_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("image.png");
    let img_part = Part::bytes(img_bytes).file_name(img_name.to_string());
    form = form.part("image", img_part);
    // optional mask file
    if let Some(mask) = mask_path {
        let mask_bytes =
            fs::read(mask).with_context(|| format!("Failed to read mask file {mask:?}"))?;
        let mask_name = mask
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("mask.png");
        let mask_part = Part::bytes(mask_bytes).file_name(mask_name.to_string());
        form = form.part("mask", mask_part);
    }
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", credentials.api_key()))
        .multipart(form)
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        bail!("OpenAI API error ({})\n{}", status, body);
    }
    let v: Value =
        serde_json::from_str(&body).with_context(|| format!("Invalid JSON response:\n{body}"))?;
    if let Some(err) = v.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        bail!("OpenAI Images API error: {}", msg);
    }
    let data = v
        .get("data")
        .with_context(|| format!("Missing 'data':\n{body}"))?;
    let items: Vec<ImageData> = serde_json::from_value(data.clone())
        .with_context(|| format!("Failed to parse 'data':\n{data:?}"))?;
    Ok(items)
}
