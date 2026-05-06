use anyhow::Result;
use clap::Parser;
use loco::backends::BackendRegistry;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(about = "MCP server for local LLMs — Ollama, LM Studio, llama.cpp")]
struct Cli {
    #[arg(long, env = "OLLAMA_URL", default_value = "http://localhost:11434")]
    ollama_url: String,

    #[arg(long, env = "LMSTUDIO_URL", default_value = "http://localhost:1234")]
    lmstudio_url: String,

    #[arg(long, env = "LLAMACPP_URL", default_value = "http://localhost:8080")]
    llamacpp_url: String,

    #[arg(long, env = "RUST_LOG", default_value = "loco=info")]
    log: String,
}

#[derive(Debug, Deserialize)]
struct Request {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

fn ok(id: &Value, result: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn err(id: &Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

async fn handle(req: Request, registry: &BackendRegistry) -> Option<Value> {
    let id = req.id.clone().unwrap_or(Value::Null);
    debug!(method = %req.method, "incoming request");

    match req.method.as_str() {
        "initialize" => {
            info!("client connected");
            Some(ok(
                &id,
                &json!({
                    "protocolVersion": "2025-06-18",
                    "capabilities":    { "tools": {} },
                    "serverInfo":      { "name": "loco", "version": env!("CARGO_PKG_VERSION") }
                }),
            ))
        }
        "notifications/initialized" => None,
        "ping" => Some(ok(&id, &json!({}))),
        "tools/list" => Some(ok(&id, &json!({ "tools": loco::tools::list() }))),
        "tools/call" => {
            let name = req.params["name"].as_str().unwrap_or("").to_string();
            let args = req.params["arguments"].clone();
            info!(tool = %name, "tool call");

            let (text, is_error) = match loco::tools::call(&name, args, registry).await {
                Ok(t) => (t, false),
                Err(e) => {
                    warn!(tool = %name, error = %e, "tool error");
                    (e.to_string(), true)
                }
            };

            Some(ok(
                &id,
                &json!({
                    "content":  [{ "type": "text", "text": text }],
                    "isError":  is_error
                }),
            ))
        }
        other => {
            warn!(method = %other, "unknown method");
            Some(err(&id, -32601, "method not found"))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::new(&cli.log))
        .init();

    info!(version = env!("CARGO_PKG_VERSION"), "loco starting");
    info!(
        ollama   = %cli.ollama_url,
        lmstudio = %cli.lmstudio_url,
        llamacpp = %cli.llamacpp_url,
        "backends configured"
    );

    let registry = BackendRegistry::new(cli.ollama_url, cli.lmstudio_url, cli.llamacpp_url);

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        if let Ok(req) = serde_json::from_str::<Request>(&line) {
            if let Some(response) = handle(req, &registry).await {
                let mut bytes = serde_json::to_vec(&response)?;
                bytes.push(b'\n');
                stdout.write_all(&bytes).await?;
                stdout.flush().await?;
            }
        } else {
            error!("malformed JSON-RPC message");
            let msg = json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": "parse error" }
            });
            let mut bytes = serde_json::to_vec(&msg)?;
            bytes.push(b'\n');
            stdout.write_all(&bytes).await?;
            stdout.flush().await?;
        }
    }

    info!("client disconnected");
    Ok(())
}
