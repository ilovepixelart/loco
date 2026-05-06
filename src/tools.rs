use std::fmt::Write as _;

use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};
use tokio_stream::StreamExt as _;
use tracing::debug;

use crate::backends::BackendRegistry;

const MAX_BUF_BYTES: usize = 10 * 1024 * 1024;
const MAX_ARG_BYTES: usize = 512 * 1024;

#[must_use]
pub fn list() -> Value {
    json!([
        {
            "name": "list_models",
            "description": "List models across running backends (Ollama, LM Studio, llama.cpp). Use this to discover what's available before calling chat or generate.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "backend": {
                        "type": "string",
                        "enum": ["ollama", "lmstudio", "llamacpp"],
                        "description": "Limit to one backend. Omit to list all."
                    }
                }
            }
        },
        {
            "name": "chat",
            "description": "Send a chat message to a local model and get a response.",
            "inputSchema": {
                "type": "object",
                "required": ["model", "messages"],
                "properties": {
                    "model":    { "type": "string", "description": "Model name (e.g. gemma3:27b, llama3.2)" },
                    "messages": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["role", "content"],
                            "properties": {
                                "role":    { "type": "string", "enum": ["system", "user", "assistant"] },
                                "content": { "type": "string" }
                            }
                        }
                    },
                    "backend":           { "type": "string", "enum": ["ollama", "lmstudio", "llamacpp"], "description": "Backend to use. Auto-detected if omitted." },
                    "temperature":       { "type": "number",  "minimum": 0,  "maximum": 2 },
                    "top_p":             { "type": "number",  "minimum": 0,  "maximum": 1 },
                    "seed":              { "type": "integer", "minimum": 0 },
                    "presence_penalty":  { "type": "number",  "minimum": -2, "maximum": 2 },
                    "frequency_penalty": { "type": "number",  "minimum": -2, "maximum": 2 },
                    "max_tokens":        { "type": "integer", "minimum": 1 }
                }
            }
        },
        {
            "name": "generate",
            "description": "Raw text completion (no chat format) from a local model.",
            "inputSchema": {
                "type": "object",
                "required": ["model", "prompt"],
                "properties": {
                    "model":             { "type": "string" },
                    "prompt":            { "type": "string" },
                    "backend":           { "type": "string", "enum": ["ollama", "lmstudio", "llamacpp"] },
                    "temperature":       { "type": "number",  "minimum": 0,  "maximum": 2 },
                    "top_p":             { "type": "number",  "minimum": 0,  "maximum": 1 },
                    "seed":              { "type": "integer", "minimum": 0 },
                    "presence_penalty":  { "type": "number",  "minimum": -2, "maximum": 2 },
                    "frequency_penalty": { "type": "number",  "minimum": -2, "maximum": 2 },
                    "max_tokens":        { "type": "integer", "minimum": 1 },
                    "stop":              { "type": "array", "items": { "type": "string" } }
                }
            }
        }
    ])
}

/// # Errors
///
/// Returns an error if the tool name is unknown or the underlying tool implementation fails.
pub async fn call(name: &str, args: Value, registry: &BackendRegistry) -> Result<String> {
    match name {
        "list_models" => do_list_models(args, registry).await,
        "chat" => do_chat(args, registry).await,
        "generate" => do_generate(args, registry).await,
        other => Err(anyhow!("unknown tool: {other}")),
    }
}

async fn do_list_models(args: Value, registry: &BackendRegistry) -> Result<String> {
    let preferred = args["backend"].as_str();
    let online = registry.online_names().await;

    if online.is_empty() {
        return Ok(
            "No backends running. Start Ollama (11434), LM Studio (1234), or llama-server (8080)."
                .into(),
        );
    }

    let targets: Vec<&str> = if let Some(name) = preferred {
        if !online.contains(&name) {
            return Ok(format!("Backend '{name}' is not running."));
        }
        vec![name]
    } else {
        online.clone()
    };

    let mut out = String::new();
    for name in targets {
        if let Some(b) = registry.find(name) {
            match registry.list_models(b).await {
                Ok(models) => {
                    let _ = writeln!(out, "\n## {} ({})", b.name, b.base_url);
                    for m in models {
                        let _ = writeln!(out, "  - {m}");
                    }
                }
                Err(e) => {
                    let _ = writeln!(out, "\n## {} — error: {e}", b.name);
                }
            }
        }
    }

    Ok(out.trim_start().to_string())
}

async fn do_chat(args: Value, registry: &BackendRegistry) -> Result<String> {
    let model = str_arg(&args, "model")?;
    let messages = args["messages"].clone();
    let backend = args["backend"].as_str();
    let max_tok = args["max_tokens"].as_u64();

    let b = registry.resolve(backend).await?;

    let sampling = build_sampling_params(&args)?;

    let mut body = json!({
        "model":    model,
        "messages": messages,
        "stream":   true,
    });
    for (k, v) in &sampling {
        body[k] = v.clone();
    }
    if let Some(mt) = max_tok {
        body["max_tokens"] = json!(mt);
    }

    let response = registry
        .client
        .post(format!("{}/v1/chat/completions", b.base_url))
        .json(&body)
        .send()
        .await?;

    collect_sse_chat(response).await
}

async fn do_generate(args: Value, registry: &BackendRegistry) -> Result<String> {
    let model = str_arg(&args, "model")?;
    let prompt = str_arg(&args, "prompt")?;
    let backend = args["backend"].as_str();
    let max_tok = args["max_tokens"].as_u64();
    let stop = &args["stop"];

    let b = registry.resolve(backend).await?;

    let sampling = build_sampling_params(&args)?;

    let mut body = json!({
        "model":  model,
        "prompt": prompt,
        "stream": true,
    });
    for (k, v) in &sampling {
        body[k] = v.clone();
    }
    if let Some(mt) = max_tok {
        body["max_tokens"] = json!(mt);
    }
    if stop.is_array() {
        body["stop"] = stop.clone();
    }

    let response = registry
        .client
        .post(format!("{}/v1/completions", b.base_url))
        .json(&body)
        .send()
        .await?;

    collect_sse_completion(response).await
}

async fn collect_sse_chat(response: reqwest::Response) -> Result<String> {
    let mut stream = response.bytes_stream();
    let mut buf = String::new();
    let mut content = String::new();

    while let Some(chunk) = stream.next().await {
        buf.push_str(std::str::from_utf8(&chunk?)?);
        if buf.len() > MAX_BUF_BYTES {
            anyhow::bail!("response exceeded {MAX_BUF_BYTES} bytes — backend may be misbehaving");
        }
        process_sse_lines(&mut buf, |data| {
            if let Some(delta) = data["choices"][0]["delta"]["content"].as_str() {
                content.push_str(delta);
            }
        });
    }

    debug!(chars = content.len(), "chat stream complete");
    Ok(content)
}

async fn collect_sse_completion(response: reqwest::Response) -> Result<String> {
    let mut stream = response.bytes_stream();
    let mut buf = String::new();
    let mut text = String::new();

    while let Some(chunk) = stream.next().await {
        buf.push_str(std::str::from_utf8(&chunk?)?);
        if buf.len() > MAX_BUF_BYTES {
            anyhow::bail!("response exceeded {MAX_BUF_BYTES} bytes — backend may be misbehaving");
        }
        process_sse_lines(&mut buf, |data| {
            if let Some(t) = data["choices"][0]["text"].as_str() {
                text.push_str(t);
            }
        });
    }

    debug!(chars = text.len(), "completion stream complete");
    Ok(text)
}

fn process_sse_lines(buf: &mut String, mut f: impl FnMut(&Value)) {
    while let Some(pos) = buf.find('\n') {
        let line = buf[..pos].trim_end_matches('\r').to_owned();
        buf.drain(..=pos);

        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                continue;
            }
            if let Ok(json) = serde_json::from_str::<Value>(data) {
                f(&json);
            }
        }
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    let s = args[key]
        .as_str()
        .ok_or_else(|| anyhow!("'{key}' is required"))?;
    if s.is_empty() {
        return Err(anyhow!("'{key}' must not be empty"));
    }
    if s.len() > MAX_ARG_BYTES {
        return Err(anyhow!("'{key}' exceeds {MAX_ARG_BYTES} bytes"));
    }
    Ok(s)
}

fn build_sampling_params(args: &Value) -> Result<Map<String, Value>> {
    let mut map = Map::new();

    if let Some(t) = args["temperature"].as_f64() {
        if !(0.0..=2.0).contains(&t) {
            return Err(anyhow!("temperature must be between 0.0 and 2.0, got {t}"));
        }
        map.insert("temperature".into(), json!(t));
    }

    if let Some(v) = args["top_p"].as_f64() {
        if !(0.0..=1.0).contains(&v) {
            return Err(anyhow!("top_p must be between 0.0 and 1.0, got {v}"));
        }
        map.insert("top_p".into(), json!(v));
    }

    if let Some(v) = args["seed"].as_u64() {
        map.insert("seed".into(), json!(v));
    }

    if let Some(v) = args["presence_penalty"].as_f64() {
        if !(-2.0..=2.0).contains(&v) {
            return Err(anyhow!(
                "presence_penalty must be between -2.0 and 2.0, got {v}"
            ));
        }
        map.insert("presence_penalty".into(), json!(v));
    }

    if let Some(v) = args["frequency_penalty"].as_f64() {
        if !(-2.0..=2.0).contains(&v) {
            return Err(anyhow!(
                "frequency_penalty must be between -2.0 and 2.0, got {v}"
            ));
        }
        map.insert("frequency_penalty".into(), json!(v));
    }

    Ok(map)
}
