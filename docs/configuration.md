# Configuration Guide

## Configuration File Location

gcop-rs uses a TOML configuration file. The location is platform-specific:

| Platform | Location |
|----------|----------|
| Linux | `~/.config/gcop/config.toml` |
| macOS | `~/Library/Application Support/gcop/config.toml` |
| Windows | `%APPDATA%\gcop\config.toml` |

The configuration file is **optional**. If not present, default values are used.

## Quick Setup

**Recommended: Use the init command**

```bash
gcop-rs init
```

This will create the config file at the correct platform-specific location.

**Manual setup:**

Linux:
```bash
mkdir -p ~/.config/gcop
cp examples/config.toml.example ~/.config/gcop/config.toml
```

macOS:
```bash
mkdir -p ~/Library/Application\ Support/gcop
cp examples/config.toml.example ~/Library/Application\ Support/gcop/config.toml
```

Windows (PowerShell):
```powershell
New-Item -ItemType Directory -Force -Path "$env:APPDATA\gcop"
Copy-Item examples\config.toml.example "$env:APPDATA\gcop\config.toml"
```

Then edit the config file to add your API key.

## Basic Configuration

Minimal configuration for Claude API:

```toml
[llm]
default_provider = "claude"

[llm.providers.claude]
api_key = "sk-ant-your-key-here"
model = "claude-sonnet-4-5-20250929"
```

## Complete Configuration Example

```toml
# LLM Configuration
[llm]
default_provider = "claude"

# Claude Provider
[llm.providers.claude]
api_key = "sk-ant-your-key"
endpoint = "https://api.anthropic.com/v1/messages"
model = "claude-sonnet-4-5-20250929"
temperature = 0.3
max_tokens = 2000

# OpenAI Provider
[llm.providers.openai]
api_key = "sk-your-openai-key"
endpoint = "https://api.openai.com/v1/chat/completions"
model = "gpt-4-turbo"
temperature = 0.3

# Ollama Provider (local)
[llm.providers.ollama]
endpoint = "http://localhost:11434/api/generate"
model = "codellama:13b"

# Commit Behavior
[commit]
show_diff_preview = true
allow_edit = true
confirm_before_commit = true
max_retries = 10

# Review Settings
[review]
show_full_diff = true
min_severity = "info"  # critical | warning | info

# UI Settings
[ui]
colored = true
verbose = false
streaming = true  # Enable streaming output (real-time typing effect)

# Note: Streaming is only supported by OpenAI-style APIs.
# For Claude and Ollama providers, it automatically falls back to spinner mode.

# Network Settings
[network]
request_timeout = 120    # HTTP request timeout in seconds
connect_timeout = 10     # HTTP connection timeout in seconds
max_retries = 3          # Max retry attempts for failed API requests
retry_delay_ms = 1000    # Initial retry delay (exponential backoff)

# File Settings
[file]
max_size = 10485760      # Max file size for review (10MB)
```

## Configuration Options

### LLM Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `default_provider` | String | `"claude"` | Default LLM provider to use |

### Provider Settings

Each provider under `[llm.providers.<name>]` supports:

| Option | Type | Required | Description |
|--------|------|----------|-------------|
| `api_style` | String | No | API style: `"claude"`, `"openai"`, or `"ollama"` (auto-detected if not set) |
| `api_key` | String | Yes* | API key (*not required for Ollama) |
| `endpoint` | String | No | API endpoint (uses default if not set) |
| `model` | String | Yes | Model name |
| `temperature` | Float | No | Temperature (0.0-1.0, default: 0.3) |
| `max_tokens` | Integer | No | Max tokens for response (default: 2000) |

### Commit Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `show_diff_preview` | Boolean | `true` | Show diff stats before generating |
| `allow_edit` | Boolean | `true` | Allow editing generated message |
| `confirm_before_commit` | Boolean | `true` | Ask confirmation before committing |
| `max_retries` | Integer | `10` | Max retry attempts for regenerating messages |
| `custom_prompt` | String | No | Custom prompt template for commit messages |

### Review Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `show_full_diff` | Boolean | `true` | Show full diff during review |
| `min_severity` | String | `"info"` | Minimum severity to display: `"critical"`, `"warning"`, or `"info"` |
| `custom_prompt` | String | No | Custom prompt template for code review |

### UI Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `colored` | Boolean | `true` | Enable colored output |
| `verbose` | Boolean | `false` | Show verbose logs (same as `--verbose` flag) |
| `streaming` | Boolean | `true` | Enable streaming output (real-time typing effect) |

> **Note on Streaming:** Currently only OpenAI-style APIs support streaming. When using Claude or Ollama providers, the system automatically falls back to spinner mode (waiting for complete response). This is transparent to the user - no configuration change needed.

### Network Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `request_timeout` | Integer | `120` | HTTP request timeout in seconds |
| `connect_timeout` | Integer | `10` | HTTP connection timeout in seconds |
| `max_retries` | Integer | `3` | Max retry attempts for failed API requests |
| `retry_delay_ms` | Integer | `1000` | Initial retry delay in milliseconds (exponential backoff) |

### File Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_size` | Integer | `10485760` | Max file size for review in bytes (default: 10MB) |

## API Key Configuration

### Priority Order

1. **Config file** (platform-specific location, see above)
2. **Environment variable** (fallback)

### Methods

**Method 1: Config File (Recommended)**

```toml
[llm.providers.claude]
api_key = "sk-ant-your-key"
```

**Method 2: Environment Variable**

```bash
export ANTHROPIC_API_KEY="sk-ant-your-key"
export OPENAI_API_KEY="sk-your-openai-key"
```

### Security

**Linux/macOS:**
- Set file permissions: `chmod 600 <config-file-path>`

**All platforms:**
- Never commit config.toml to git
- Add to .gitignore if creating project-level config

## Override with Command-Line

```bash
# Override provider
gcop-rs --provider openai commit

# Enable verbose mode
gcop-rs -v commit
```

Command-line options override configuration file.

## See Also

- [Provider Setup](providers.md) - Configure LLM providers
- [Custom Prompts](prompts.md) - Customize AI prompts
- [Troubleshooting](troubleshooting.md) - Common configuration issues
