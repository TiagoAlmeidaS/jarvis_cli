# Jarvis CLI

<p align="center">Lightweight coding agent that runs in your terminal</p>

<p align="center"><code>npm i -g @jarvis/cli</code></p>

> [!IMPORTANT]
> This is the documentation for the _legacy_ TypeScript implementation of the Jarvis CLI. It has been superseded by the _Rust_ implementation. See the [README in the root of the Jarvis repository](https://github.com/TiagoAlmeidaS/jarvis-cli/blob/main/README.md) for details.

![Jarvis demo GIF](../.github/codex-cli-splash.png)

---

## About

**Jarvis CLI** is a coding agent that runs locally on your computer. It's a fork of the OpenAI Codex CLI, customized and maintained by [TiagoAlmeidaS](https://github.com/TiagoAlmeidaS).

## Quickstart

### Installation

Install globally with npm:

```bash
npm install -g @jarvis/cli
```

Or using other package managers:

```bash
# Using yarn
yarn global add @jarvis/cli

# Using pnpm
pnpm add -g @jarvis/cli

# Using bun
bun install -g @jarvis/cli
```

### Usage

Simply run:

```bash
jarvis
```

Or with an initial prompt:

```bash
jarvis "explain this codebase to me"
```

## Configuration

Jarvis CLI uses a `config.toml` file for configuration. The configuration file should be placed in:

- **User configuration:** `~/.jarvis/config.toml` (Windows: `C:\Users\<user>\.jarvis\config.toml`)
- **Project configuration:** `.jarvis/config.toml` (in your project root)

### Example Configuration

See `config.toml.example` in the repository root for a complete example configuration.

### Model Providers

Jarvis CLI supports multiple LLM providers:

- **Databricks** (default)
- **OpenAI**
- **Azure OpenAI**
- **OpenRouter**
- **Anthropic**
- **Ollama** (local)

Configure providers in `config.toml`:

```toml
[model_providers.databricks]
name = "Databricks"
base_url = "https://your-workspace.cloud.databricks.com/serving-endpoints/your-endpoint/invocations"
env_key = "DATABRICKS_API_KEY"
wire_api = "responses"
http_headers = { "Content-Type" = "application/json" }

[profiles.default]
model_provider = "databricks"
model = "databricks-claude-opus-4-5"
```

### Environment Variables

Set API keys as environment variables:

```bash
# Windows PowerShell
[System.Environment]::SetEnvironmentVariable("DATABRICKS_API_KEY", "your-key-here", "User")

# Linux/macOS
export DATABRICKS_API_KEY="your-key-here"
```

## Development

This package is a wrapper around the Rust implementation of Jarvis CLI. The Rust binaries are installed automatically when you install this npm package.

### Building from Source

See the main [README.md](../README.md) for instructions on building from source.

## Repository

- **GitHub:** https://github.com/TiagoAlmeidaS/jarvis_cli
- **Original Project:** https://github.com/openai/codex (OpenAI Codex CLI)

## License

Apache-2.0 License

## Contributing

Contributions are welcome! Please see the main repository's [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

---

**Note:** This is the legacy TypeScript wrapper. The main implementation is in Rust and can be found in the `jarvis-rs/` directory.
