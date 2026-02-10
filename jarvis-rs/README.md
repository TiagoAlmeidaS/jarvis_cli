# jarvis CLI (Rust Implementation)

We provide jarvis CLI as a standalone, native executable to ensure a zero-dependency install.

## Installing jarvis

Today, the easiest way to install jarvis is via `npm`:

```shell
npm i -g @openai/jarvis
jarvis
```

You can also install via Homebrew (`brew install --cask jarvis`) or download a platform-specific release directly from our [GitHub Releases](https://github.com/openai/jarvis/releases).

## Documentation quickstart

- First run with jarvis? Start with [`docs/getting-started.md`](../docs/getting-started.md) (links to the walkthrough for prompts, keyboard shortcuts, and session management).
- Want deeper control? See [`docs/config.md`](../docs/config.md) and [`docs/install.md`](../docs/install.md).

## What's new in the Rust CLI

The Rust implementation is now the maintained jarvis CLI and serves as the default experience. It includes a number of features that the legacy TypeScript CLI never supported.

### Config

jarvis supports a rich set of configuration options. Note that the Rust CLI uses `config.toml` instead of `config.json`. See [`docs/config.md`](../docs/config.md) for details.

### Model Context Protocol Support

#### MCP client

jarvis CLI functions as an MCP client that allows the jarvis CLI and IDE extension to connect to MCP servers on startup. See the [`configuration documentation`](../docs/config.md#connecting-to-mcp-servers) for details.

#### MCP server (experimental)

jarvis can be launched as an MCP _server_ by running `jarvis mcp-server`. This allows _other_ MCP clients to use jarvis as a tool for another agent.

Use the [`@modelcontextprotocol/inspector`](https://github.com/modelcontextprotocol/inspector) to try it out:

```shell
npx @modelcontextprotocol/inspector jarvis mcp-server
```

Use `jarvis mcp` to add/list/get/remove MCP server launchers defined in `config.toml`, and `jarvis mcp-server` to run the MCP server directly.

### Notifications

You can enable notifications by configuring a script that is run whenever the agent finishes a turn. The [notify documentation](../docs/config.md#notify) includes a detailed example that explains how to get desktop notifications via [terminal-notifier](https://github.com/julienXX/terminal-notifier) on macOS. When jarvis detects that it is running under WSL 2 inside Windows Terminal (`WT_SESSION` is set), the TUI automatically falls back to native Windows toast notifications so approval prompts and completed turns surface even though Windows Terminal does not implement OSC 9.

### `jarvis exec` to run jarvis programmatically/non-interactively

To run jarvis non-interactively, run `jarvis exec PROMPT` (you can also pass the prompt via `stdin`) and jarvis will work on your task until it decides that it is done and exits. Output is printed to the terminal directly. You can set the `RUST_LOG` environment variable to see more about what's going on.

### Experimenting with the jarvis Sandbox

To test to see what happens when a command is run under the sandbox provided by jarvis, we provide the following subcommands in jarvis CLI:

```
# macOS
jarvis sandbox macos [--full-auto] [--log-denials] [COMMAND]...

# Linux
jarvis sandbox linux [--full-auto] [COMMAND]...

# Windows
jarvis sandbox windows [--full-auto] [COMMAND]...

# Legacy aliases
jarvis debug seatbelt [--full-auto] [--log-denials] [COMMAND]...
jarvis debug landlock [--full-auto] [COMMAND]...
```

### Selecting a sandbox policy via `--sandbox`

The Rust CLI exposes a dedicated `--sandbox` (`-s`) flag that lets you pick the sandbox policy **without** having to reach for the generic `-c/--config` option:

```shell
# Run jarvis with the default, read-only sandbox
jarvis --sandbox read-only

# Allow the agent to write within the current workspace while still blocking network access
jarvis --sandbox workspace-write

# Danger! Disable sandboxing entirely (only do this if you are already running in a container or other isolated env)
jarvis --sandbox danger-full-access
```

The same setting can be persisted in `~/.jarvis/config.toml` via the top-level `sandbox_mode = "MODE"` key, e.g. `sandbox_mode = "workspace-write"`.

## Code Organization

This folder is the root of a Cargo workspace. It contains quite a bit of experimental code, but here are the key crates:

- [`core/`](./core) contains the business logic for jarvis. Ultimately, we hope this to be a library crate that is generally useful for building other Rust/native applications that use jarvis.
- [`exec/`](./exec) "headless" CLI for use in automation.
- [`tui/`](./tui) CLI that launches a fullscreen TUI built with [Ratatui](https://ratatui.rs/).
- [`cli/`](./cli) CLI multitool that provides the aforementioned CLIs via subcommands.
