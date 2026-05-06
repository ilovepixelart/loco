# 🚂 loco

MCP server for local LLMs. Single Rust binary that bridges Claude Desktop, Cursor, Zed, or any MCP client to **Ollama**, **LM Studio**, and **llama.cpp** running on your machine.

## Install

### Cargo

    cargo install --git https://github.com/ilovepixelart/loco

### Pre-built binary

Grab the binary for your platform from the [latest release](https://github.com/ilovepixelart/loco/releases/latest):

- `loco-aarch64-apple-darwin` — macOS Apple Silicon
- `loco-x86_64-apple-darwin` — macOS Intel
- `loco-x86_64-unknown-linux-musl` — Linux x86_64
- `loco-aarch64-unknown-linux-musl` — Linux ARM64
- `loco-x86_64-pc-windows-msvc.exe` — Windows

Then `chmod +x` and move it onto your `PATH`.

### Build from source

    git clone https://github.com/ilovepixelart/loco
    cd loco
    cargo build --release
    ./target/release/loco --help

### Docker

    docker run --rm -i \
      -e OLLAMA_URL=http://host.docker.internal:11434 \
      ghcr.io/ilovepixelart/loco:latest

## Wire it up

### Claude Desktop

`~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

    {
      "mcpServers": {
        "loco": {
          "command": "loco",
          "env": {
            "OLLAMA_URL":   "http://localhost:11434",
            "LMSTUDIO_URL": "http://localhost:1234",
            "LLAMACPP_URL": "http://localhost:8080"
          }
        }
      }
    }

### Cursor — `.cursor/mcp.json`

    { "mcpServers": { "loco": { "command": "loco" } } }

### Zed — `settings.json`

    { "context_servers": { "loco": { "command": { "path": "loco", "args": [] } } } }

## Tools

| Tool          | Description                                              |
| ------------- | -------------------------------------------------------- |
| `list_models` | Enumerate models across every running backend            |
| `chat`        | Chat completion — multi-turn, with sampling controls     |
| `generate`    | Raw text completion — no chat format                     |

## Configuration

| Flag             | Env var        | Default                  |
| ---------------- | -------------- | ------------------------ |
| `--ollama-url`   | `OLLAMA_URL`   | `http://localhost:11434` |
| `--lmstudio-url` | `LMSTUDIO_URL` | `http://localhost:1234`  |
| `--llamacpp-url` | `LLAMACPP_URL` | `http://localhost:8080`  |
| `--log`          | `RUST_LOG`     | `loco=info`              |

Logs go to stderr — stdout is reserved for MCP JSON-RPC.

## License

MIT — see [LICENSE](LICENSE).
