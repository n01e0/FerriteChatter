use anyhow::{Context, Result};
use futures_util::StreamExt;
use openai::Credentials;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone)]
pub struct WebSearchClient {
    client: Client,
}

impl WebSearchClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn stream_response<F>(
        &self,
        credentials: &Credentials,
        model: &str,
        messages: &[WebMessage],
        use_tools: bool,
        on_delta: F,
        verbose: bool,
    ) -> Result<WebSearchResult>
    where
        F: FnMut(&str) -> Result<()> + Send,
    {
        if use_tools {
            let tools = Some(vec![ToolSpecification {
                r#type: ToolType::WebSearch,
            }]);
            self.stream_responses(credentials, model, messages, tools, on_delta, verbose)
                .await
        } else {
            self.stream_chat_model(credentials, model, messages, on_delta, verbose)
                .await
        }
    }

    async fn stream_responses<F>(
        &self,
        credentials: &Credentials,
        model: &str,
        messages: &[WebMessage],
        tools: Option<Vec<ToolSpecification>>,
        mut on_delta: F,
        verbose: bool,
    ) -> Result<WebSearchResult>
    where
        F: FnMut(&str) -> Result<()> + Send,
    {
        let url = format!("{}/responses", credentials.base_url());
        let body = ResponsesRequest {
            model: model.to_string(),
            input: messages
                .iter()
                .map(|m| {
                    let content_type = if m.role == "assistant" {
                        "output_text"
                    } else {
                        "input_text"
                    };
                    ResponseMessage {
                        role: m.role.clone(),
                        content: vec![ResponseContent {
                            kind: content_type.to_string(),
                            text: m.content.clone(),
                        }],
                    }
                })
                .collect(),
            tools,
        };

        let response = self
            .client
            .post(&url)
            .query(&[("stream", "true")])
            .header("Authorization", format!("Bearer {}", credentials.api_key()))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .with_context(|| "Failed to send responses request")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Web search API error ({}): {}",
                status,
                text
            ));
        }

        let mut text_buffer = String::new();
        let mut final_response: Option<Value> = None;
        let mut final_text = String::new();
        let mut final_citation_values: Vec<Value> = Vec::new();
        let mut stream = response.bytes_stream();
        let mut carry = String::new();
        let mut citation_values: Vec<Value> = Vec::new();
        let mut displayed = false;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.with_context(|| "Failed to read response chunk")?;
            let piece = String::from_utf8_lossy(&bytes);
            if verbose {
                eprintln!("[responses chunk] {}", piece);
            }
            carry.push_str(&piece.replace("\r\n", "\n"));

            while let Some(idx) = carry.find("\n\n") {
                let event = carry[..idx].to_string();
                carry = carry[idx + 2..].to_string();
                if let Some(line) = event.lines().find(|l| l.starts_with("data:")) {
                    let payload = line.trim_start_matches("data:").trim();
                    if payload == "[DONE]" {
                        break;
                    }
                    if payload.is_empty() {
                        continue;
                    }
                    let json: Value =
                        serde_json::from_str(payload).with_context(|| "Invalid JSON chunk")?;
                    collect_possible_citations(&json, &mut citation_values);

                    if let Some(event_type) = json.get("type").and_then(|v| v.as_str()) {
                        match event_type {
                            "response.output_text.delta" => {
                                if let Some(delta_val) = json.get("delta") {
                                    if handle_delta_value(
                                        delta_val,
                                        &mut on_delta,
                                        &mut text_buffer,
                                    )? {
                                        displayed = true;
                                    }
                                    collect_possible_citations(delta_val, &mut citation_values);
                                    if verbose {
                                        eprintln!(
                                            "[responses delta] {}",
                                            serde_json::to_string(delta_val).unwrap_or_default()
                                        );
                                    }
                                }
                            }
                            t if t.starts_with("response.output_text.annotation") => {
                                if let Some(annotation) = json.get("annotation") {
                                    citation_values.push(annotation.clone());
                                    if verbose {
                                        eprintln!(
                                            "[responses annotation] {}",
                                            serde_json::to_string(annotation).unwrap_or_default()
                                        );
                                    }
                                }
                            }
                            "response.output_text" => {
                                if let Some(output) = json.get("output") {
                                    for segment in extract_text_segments_list(output) {
                                        if !segment.is_empty() {
                                            on_delta(&segment)?;
                                            text_buffer.push_str(&segment);
                                            displayed = true;
                                        }
                                    }
                                    citation_values.push(output.clone());
                                    if verbose {
                                        eprintln!(
                                            "[responses output] {}",
                                            serde_json::to_string(output).unwrap_or_default()
                                        );
                                    }
                                }
                            }
                            "response.completed" => {
                                if let Some(resp) = json.get("response") {
                                    final_response = Some(resp.clone());
                                    citation_values.push(resp.clone());
                                    if verbose {
                                        eprintln!(
                                            "[responses completed] {}",
                                            serde_json::to_string(resp).unwrap_or_default()
                                        );
                                    }
                                }
                            }
                            "message" => {
                                if let Some(content_text) = extract_text_from_response(&json) {
                                    if text_buffer.is_empty() {
                                        text_buffer = content_text.clone();
                                    }
                                    if !content_text.is_empty() && !displayed {
                                        on_delta(&content_text)?;
                                        displayed = true;
                                    }
                                    final_text = content_text;
                                } else {
                                    citation_values.push(json.clone());
                                }
                                if final_response.is_none() {
                                    final_response = Some(json.clone());
                                }
                            }
                            "response.error" => {
                                let message = json
                                    .get("error")
                                    .and_then(|e| e.get("message"))
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("Unknown error");
                                return Err(anyhow::anyhow!(message.to_string()));
                            }
                            _ => { /* ignore other events */ }
                        }
                    } else if json.get("output").is_some() {
                        if verbose {
                            eprintln!(
                                "[responses full] {}",
                                serde_json::to_string(&json).unwrap_or_default()
                            );
                        }
                        let (text, cites) = parse_response_output(&json);
                        if !text.is_empty() {
                            final_text = text;
                        }
                        if !cites.is_empty() {
                            final_citation_values = cites;
                        }
                        if verbose {
                            eprintln!("[responses full parsed]");
                        }
                        final_response = Some(json.clone());
                    }
                }
            }
        }

        if final_text.is_empty() {
            if text_buffer.trim().is_empty() {
                if let Some(resp) = final_response.as_ref() {
                    let (text, cites) = parse_response_output(resp);
                    if !text.is_empty() {
                        final_text = text;
                    } else if let Some(fallback) = extract_text_from_response(resp) {
                        final_text = fallback;
                    }
                    if !cites.is_empty() {
                        final_citation_values = cites;
                    }
                }
            } else {
                final_text = text_buffer.clone();
            }
        }

        let mut citations = Vec::new();
        let mut seen = HashSet::new();
        for value in citation_values.iter().chain(final_citation_values.iter()) {
            collect_citations(value, &mut citations, &mut seen);
        }

        Ok(WebSearchResult {
            message: final_text,
            citations,
            displayed,
        })
    }

    async fn stream_chat_model<F>(
        &self,
        credentials: &Credentials,
        model: &str,
        messages: &[WebMessage],
        mut on_delta: F,
        verbose: bool,
    ) -> Result<WebSearchResult>
    where
        F: FnMut(&str) -> Result<()> + Send,
    {
        let url = format!("{}/chat/completions", credentials.base_url());
        let body = ChatCompletionRequest {
            model: model.to_string(),
            messages: messages
                .iter()
                .map(|m| ChatMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect(),
            stream: true,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", credentials.api_key()))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .with_context(|| "Failed to send chat completion request")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Web search API error ({}): {}",
                status,
                text
            ));
        }

        let mut text_buffer = String::new();
        let mut carry = String::new();
        let mut stream = response.bytes_stream();
        let mut citation_values: Vec<Value> = Vec::new();
        let mut final_message: Option<Value> = None;
        let mut displayed = false;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.with_context(|| "Failed to read response chunk")?;
            let piece = String::from_utf8_lossy(&bytes);
            if verbose {
                eprintln!("[chat chunk] {}", piece);
            }
            carry.push_str(&piece.replace("\r\n", "\n"));

            while let Some(idx) = carry.find("\n\n") {
                let event = carry[..idx].to_string();
                carry = carry[idx + 2..].to_string();
                if let Some(line) = event.lines().find(|l| l.starts_with("data:")) {
                    let payload = line.trim_start_matches("data:").trim();
                    if payload == "[DONE]" {
                        break;
                    }
                    if payload.is_empty() {
                        continue;
                    }
                    let json: Value =
                        serde_json::from_str(payload).with_context(|| "Invalid JSON chunk")?;
                    if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                        if let Some(choice) = choices.first() {
                            if let Some(delta) = choice.get("delta") {
                                process_chat_delta(
                                    delta,
                                    &mut text_buffer,
                                    &mut citation_values,
                                    &mut on_delta,
                                    &mut displayed,
                                )?;
                                if verbose {
                                    eprintln!(
                                        "[chat delta] {}",
                                        serde_json::to_string(delta).unwrap_or_default()
                                    );
                                }
                            }
                            if let Some(message) = choice.get("message") {
                                final_message = Some(message.clone());
                            }
                            if choice
                                .get("finish_reason")
                                .and_then(|f| f.as_str())
                                .is_some()
                            {
                                if let Some(message) = choice.get("message") {
                                    final_message = Some(message.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(message) = final_message {
            if text_buffer.is_empty() {
                if let Some(content) = extract_text_from_message(&message) {
                    if !content.is_empty() {
                        text_buffer = content;
                    }
                }
            }
            citation_values.push(message);
        }

        let mut citations = Vec::new();
        let mut seen = HashSet::new();
        for value in citation_values {
            collect_citations(&value, &mut citations, &mut seen);
        }

        if !displayed && !text_buffer.is_empty() {
            on_delta(&text_buffer)?;
            displayed = true;
        }

        Ok(WebSearchResult {
            message: text_buffer,
            citations,
            displayed,
        })
    }
}

#[derive(Clone)]
pub struct WebMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct Citation {
    pub url: String,
    pub title: Option<String>,
}

pub struct WebSearchResult {
    pub message: String,
    pub citations: Vec<Citation>,
    pub displayed: bool,
}

#[derive(Serialize)]
struct ResponsesRequest {
    model: String,
    input: Vec<ResponseMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolSpecification>>,
}

#[derive(Serialize)]
struct ResponseMessage {
    role: String,
    content: Vec<ResponseContent>,
}

#[derive(Serialize)]
struct ResponseContent {
    #[serde(rename = "type")]
    kind: String,
    text: String,
}

#[derive(Serialize)]
struct ToolSpecification {
    #[serde(rename = "type")]
    r#type: ToolType,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ToolType {
    WebSearch,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

fn collect_citations(value: &Value, citations: &mut Vec<Citation>, seen: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            let url_field = map
                .get("url")
                .or_else(|| map.get("source_url"))
                .or_else(|| map.get("href"))
                .or_else(|| map.get("uri"))
                .and_then(|u| u.as_str());
            if let Some(url) = url_field {
                let key = url.to_string();
                if seen.insert(key.clone()) {
                    let title = map
                        .get("title")
                        .or_else(|| map.get("name"))
                        .or_else(|| map.get("source"))
                        .or_else(|| map.get("page_title"))
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string());
                    citations.push(Citation { url: key, title });
                }
            }
            for v in map.values() {
                collect_citations(v, citations, seen);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                collect_citations(item, citations, seen);
            }
        }
        _ => {}
    }
}

fn handle_delta_value<F>(
    delta_val: &Value,
    on_delta: &mut F,
    text_buffer: &mut String,
) -> Result<bool>
where
    F: FnMut(&str) -> Result<()> + Send,
{
    let mut emitted = false;
    match delta_val {
        Value::String(s) => {
            if !s.is_empty() {
                on_delta(s)?;
                text_buffer.push_str(s);
                emitted = true;
            }
        }
        _ => {
            for segment in extract_text_segments_list(delta_val) {
                if !segment.is_empty() {
                    on_delta(&segment)?;
                    text_buffer.push_str(&segment);
                    emitted = true;
                }
            }
        }
    }
    Ok(emitted)
}

fn collect_possible_citations(value: &Value, collector: &mut Vec<Value>) {
    match value {
        Value::Object(_) | Value::Array(_) => collector.push(value.clone()),
        _ => {}
    }
}

fn parse_response_output(value: &Value) -> (String, Vec<Value>) {
    let mut text = String::new();
    let mut citations = Vec::new();

    if let Some(output) = value.get("output").and_then(|o| o.as_array()) {
        for item in output {
            collect_possible_citations(item, &mut citations);
            let item_type = item.get("type").and_then(|t| t.as_str());
            if item_type != Some("message") {
                continue;
            }

            if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
                for part in content {
                    match part.get("type").and_then(|t| t.as_str()) {
                        Some("output_text") => {
                            if let Some(s) = part.get("text").and_then(|t| t.as_str()) {
                                text.push_str(s);
                            }
                            if let Some(ann) = part.get("annotations") {
                                citations.push(ann.clone());
                            }
                        }
                        _ => {
                            if let Some(s) = part.get("text").and_then(|t| t.as_str()) {
                                text.push_str(s);
                            } else if let Some(s) = part.as_str() {
                                text.push_str(s);
                            }
                        }
                    }
                    collect_possible_citations(part, &mut citations);
                }
            }
        }
    }

    (text, citations)
}

fn process_chat_delta<F>(
    delta: &Value,
    text_buffer: &mut String,
    citation_values: &mut Vec<Value>,
    on_delta: &mut F,
    displayed: &mut bool,
) -> Result<()>
where
    F: FnMut(&str) -> Result<()> + Send,
{
    if let Some(content) = delta.get("content") {
        match content {
            Value::Array(items) => {
                for item in items {
                    if handle_delta_value(item, on_delta, text_buffer)? {
                        *displayed = true;
                    }
                    collect_possible_citations(item, citation_values);
                }
            }
            Value::String(s) => {
                if !s.is_empty() {
                    on_delta(s)?;
                    text_buffer.push_str(s);
                    *displayed = true;
                }
            }
            other => {
                if handle_delta_value(other, on_delta, text_buffer)? {
                    *displayed = true;
                }
                collect_possible_citations(other, citation_values);
            }
        }
    }

    if let Some(citations) = delta.get("citations") {
        citation_values.push(citations.clone());
    }
    if let Some(annotations) = delta.get("annotations") {
        citation_values.push(annotations.clone());
    }
    if let Some(metadata) = delta.get("metadata") {
        citation_values.push(metadata.clone());
    }

    Ok(())
}

fn extract_text_from_message(message: &Value) -> Option<String> {
    if let Some(content) = message.get("content") {
        match content {
            Value::String(s) => return Some(s.to_string()),
            Value::Array(items) => {
                let mut segments = Vec::new();
                for item in items {
                    segments.extend(extract_text_segments_list(item));
                }
                if !segments.is_empty() {
                    return Some(segments.join("\n\n"));
                }
            }
            other => {
                let segments = extract_text_segments_list(other);
                if !segments.is_empty() {
                    return Some(segments.join("\n\n"));
                }
            }
        }
    }
    None
}

fn extract_text_from_response(value: &Value) -> Option<String> {
    let segments = extract_text_segments_list(value);
    if segments.is_empty() {
        None
    } else {
        Some(segments.join("\n\n"))
    }
}

fn collect_text_segments(value: &Value, segments: &mut Vec<String>) {
    match value {
        Value::String(_) => {}
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(|v| v.as_str()) {
                let ty = map.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if ty.is_empty() || matches!(ty, "output_text" | "text" | "summary_text" | "output")
                {
                    segments.push(text.to_string());
                }
            }
            if let Some(delta) = map.get("text_delta").and_then(|v| v.as_str()) {
                segments.push(delta.to_string());
            }
            for (key, val) in map.iter() {
                match key.as_str() {
                    "text" | "text_delta" => continue,
                    "content" | "messages" | "output" | "choices" | "items" | "parts" => {
                        collect_text_segments(val, segments);
                    }
                    _ => {
                        if val.is_array() || val.is_object() {
                            collect_text_segments(val, segments);
                        }
                    }
                }
            }
        }
        Value::Array(arr) => {
            for item in arr {
                collect_text_segments(item, segments);
            }
        }
        _ => {}
    }
}

fn extract_text_segments_list(value: &Value) -> Vec<String> {
    let mut segments = Vec::new();
    collect_text_segments(value, &mut segments);
    segments.into_iter().filter(|seg| !seg.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    #[test]
    fn extract_text_segments_handles_text_delta() {
        let value = json!({
            "type": "response.output_text.delta",
            "delta": {
                "content": [
                    {"type": "output_text", "text_delta": "Short answer: "},
                    {"type": "output_text", "text_delta": "n01e0 is here.\n"}
                ]
            }
        });
        let segments = extract_text_segments_list(&value);
        assert_eq!(segments, vec!["Short answer: ", "n01e0 is here.\n"]);
    }

    #[test]
    fn handle_delta_value_emits_text_delta() {
        let delta = json!({
            "content": [
                {"type": "output_text", "text_delta": "hello "},
                {"type": "output_text", "text_delta": "world"}
            ]
        });
        let mut buffer = String::new();
        let mut captured = String::new();
        let emitted = handle_delta_value(
            &delta,
            &mut |chunk| {
                captured.push_str(chunk);
                Ok(())
            },
            &mut buffer,
        )
        .expect("handle_delta_value should succeed");
        assert!(emitted);
        assert_eq!(buffer, "hello world");
        assert_eq!(captured, "hello world");
    }

    #[test]
    fn parse_response_output_extracts_text_and_citations() {
        let response = json!({
            "output": [
                {
                    "type": "message",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Short answer: example text.",
                            "annotations": [
                                {
                                    "type": "url_citation",
                                    "url": "https://example.com",
                                    "title": "Example Title"
                                }
                            ]
                        }
                    ]
                }
            ]
        });
        let (text, citation_values) = parse_response_output(&response);
        assert!(
            text.contains("Short answer: example text."),
            "parsed text should contain the response body"
        );

        let mut citations = Vec::new();
        let mut seen = HashSet::new();
        for value in citation_values {
            collect_citations(&value, &mut citations, &mut seen);
        }
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].url, "https://example.com");
        assert_eq!(citations[0].title.as_deref(), Some("Example Title"));
    }

    #[test]
    fn message_event_citations_are_collectible() {
        let event = json!({
            "type": "message",
            "content": [
                {
                    "type": "output_text",
                    "text": "Example with inline cite.",
                    "annotations": [
                        {
                            "type": "url_citation",
                            "url": "https://example.org",
                            "title": "Example Org"
                        }
                    ]
                }
            ]
        });

        let mut collected = Vec::new();
        collect_possible_citations(&event, &mut collected);

        let mut citations = Vec::new();
        let mut seen = HashSet::new();
        for value in collected {
            collect_citations(&value, &mut citations, &mut seen);
        }
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].url, "https://example.org");
        assert_eq!(citations[0].title.as_deref(), Some("Example Org"));
    }
}
