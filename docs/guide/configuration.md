# Configuration Guide

## Configuration Files

gcop-rs reads TOML configuration from two levels:

### User-Level Config

This is your personal config (usually contains API keys):

| Platform | Location |
|----------|----------|
| Linux | `~/.config/gcop/config.toml` |
| macOS | `~/Library/Application Support/gcop/config.toml` |
| Windows | `%APPDATA%\gcop\config\config.toml` |

### Project-Level Config (Optional)

Team-shared config in your repository:

| Scope | Location |
|-------|----------|
| Project | `<repo>/.gcop/config.toml` |

`gcop-rs` searches upward from the current working directory and stops at the nearest `.git` boundary.

### Effective Priority (High → Low)

1. CI overrides (`CI=1` + `GCOP_CI_*`)
2. Environment overrides (`GCOP__*`)
3. Project-level config (`.gcop/config.toml`)
4. User-level config (platform-specific path above)
5. Built-in defaults

All config files are **optional**. Missing values fall back to lower-priority sources/defaults.

## Quick Setup

**Recommended: Use init commands**

```bash
gcop-rs init
gcop-rs init --project   # optional: create .gcop/config.toml for team-shared settings
```

`gcop-rs init` creates your user-level config at the correct platform-specific location.
`gcop-rs init --project` creates `.gcop/config.toml` in the current repository.

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
New-Item -ItemType Directory -Force -Path "$env:APPDATA\gcop\config"
Copy-Item examples\config.toml.example "$env:APPDATA\gcop\config\config.toml"
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
# fallback_providers = ["openai", "gemini", "ollama"]  # Auto-fallback when main provider fails
max_diff_size = 102400  # Max diff bytes sent to LLM before truncation

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

# Gemini Provider
[llm.providers.gemini]
api_key = "AIza-your-gemini-key"
model = "gemini-3-flash-preview"

# Commit Behavior
[commit]
show_diff_preview = true
allow_edit = true
max_retries = 10

# Optional commit convention guidance (prompt-level)
[commit.convention]
style = "conventional"  # conventional | gitmoji | custom
types = ["feat", "fix", "docs", "refactor", "test", "chore"]
template = "{type}({scope}): {subject}"  # useful with style = "custom"
extra_prompt = "Commit subject should be in English"

# Review Settings
[review]
min_severity = "info"  # critical | warning | info (applies to text output)

# UI Settings
[ui]
colored = true
streaming = true  # Enable streaming output (real-time typing effect)
language = "en"  # Optional: force UI language (e.g., "en", "zh-CN")

# Note: Streaming is supported by OpenAI-, Claude-, and Gemini-style APIs.
# For Ollama providers, it automatically falls back to spinner mode.

# Network Settings
[network]
request_timeout = 120    # HTTP request timeout in seconds
connect_timeout = 10     # HTTP connection timeout in seconds
max_retries = 3          # Max retry attempts for failed API requests
retry_delay_ms = 1000    # Initial retry delay (exponential backoff)
max_retry_delay_ms = 60000  # Max retry delay; also limits Retry-After header

# File Settings
[file]
max_size = 10485760      # Max file size for `review file <PATH>` (10MB)

# Workspace Settings (monorepo scope inference)
[workspace]
enabled = true
members = ["packages/*", "apps/*"]  # Optional: override auto-detection
scope_mappings = { "packages/core" = "core", "packages/ui" = "ui" }
```

## Configuration Options

### LLM Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `default_provider` | String | `"claude"` | Default LLM provider to use |
| `fallback_providers` | Array | `[]` | Fallback provider list; automatically tries next when main provider fails |
| `max_diff_size` | Integer | `102400` | Maximum diff size (bytes) sent to LLM; larger inputs are truncated |

### Provider Settings

Each provider under `[llm.providers.<name>]` supports:

| Option | Type | Required | Description |
|--------|------|----------|-------------|
| `api_style` | String | No | API style: `"claude"`, `"openai"`, `"ollama"`, or `"gemini"` (defaults to provider name if not set) |
| `api_key` | String | Yes* | API key (*not required for Ollama) |
| `endpoint` | String | No | API endpoint (uses default if not set) |
| `model` | String | Yes | Model name |
| `temperature` | Float | No | Temperature (0.0-2.0). Claude/OpenAI/Gemini-style defaults to 0.3; Ollama uses provider default when omitted |
| `max_tokens` | Integer | No | Max response tokens. Claude-style defaults to 2000; OpenAI-style sends only if set; Ollama currently ignores this field |
| `extra` | Object | No | Additional provider-specific keys. Unknown keys are preserved; `max_tokens`/`temperature` are also read from here as a compatibility fallback |

### Commit Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `show_diff_preview` | Boolean | `true` | Show diff stats before generating |
| `allow_edit` | Boolean | `true` | Allow editing generated message |
| `max_retries` | Integer | `10` | Max generation attempts (including the first generation) |
| `custom_prompt` | String | No | Custom system prompt / instructions for commit generation |
| `convention` | Table | No | Optional prompt-level convention guidance; see `[commit.convention]` below |

### Commit Convention Settings (`[commit.convention]`)

These settings are prompt-level guidance for commit generation. They influence model output but are not hard validation rules.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `style` | String | `"conventional"` | Convention style: `"conventional"`, `"gitmoji"`, or `"custom"` |
| `types` | Array | No | Allowed commit types (mainly for `conventional` / `custom`) |
| `template` | String | No | Custom template hint (for example `{type}({scope}): {subject}`) |
| `extra_prompt` | String | No | Additional plain-text instruction appended to convention guidance |

### Review Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `min_severity` | String | `"info"` | Minimum severity to display in **text output**: `"critical"`, `"warning"`, or `"info"` |
| `custom_prompt` | String | No | Custom system prompt / instructions for code review |

### UI Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `colored` | Boolean | `true` | Enable colored output |
| `streaming` | Boolean | `true` | Enable streaming output (real-time typing effect) |
| `language` | String | `null` (auto) | Force UI language (e.g., `"en"`, `"zh-CN"`); if unset, gcop-rs auto-detects |

> **Legacy Keys:** Older config files may still contain keys such as `commit.confirm_before_commit`, `review.show_full_diff`, or `ui.verbose`. These keys are currently ignored.

> **Note on Streaming:** OpenAI, Claude, and Gemini style APIs support streaming. When using Ollama providers, the system automatically falls back to spinner mode (waiting for complete response).

### Network Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `request_timeout` | Integer | `120` | HTTP request timeout in seconds |
| `connect_timeout` | Integer | `10` | HTTP connection timeout in seconds |
| `max_retries` | Integer | `3` | Max retry attempts for failed API requests |
| `retry_delay_ms` | Integer | `1000` | Initial retry delay in milliseconds (exponential backoff) |
| `max_retry_delay_ms` | Integer | `60000` | Max retry delay in ms; also limits Retry-After header |

### File Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_size` | Integer | `10485760` | Max file size in bytes when using `review file <PATH>` (default: 10MB) |

### Workspace Settings

Workspace settings control monorepo detection and commit scope inference.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | Boolean | `true` | Enable workspace detection and scope inference |
| `members` | Array | No | Optional member patterns to use directly (skips auto-detection when set) |
| `scope_mappings` | Object | `{}` | Optional path-to-scope remap (for example `"packages/core" = "core"`) |

Auto-detection currently recognizes Cargo workspace, pnpm workspace, npm/yarn workspaces, Lerna, Nx, and Turborepo structures.

## API Key Configuration

### Sources

- **User-level config file** (platform-specific location, see above)
- **Project-level config file** (`.gcop/config.toml`, optional, for non-secret team settings)
- **CI mode environment variables** (`GCOP_CI_*`, only when `CI=1`)

When `CI=1`, CI-mode provider settings are applied after file/env loading, and become the effective default provider (`ci`).

### Methods

**Method 1: Config File (Recommended)**

```toml
[llm.providers.claude]
api_key = "sk-ant-your-key"
```

**Method 2: CI Mode Environment Variables**

```bash
export CI=1
export GCOP_CI_PROVIDER=claude
export GCOP_CI_API_KEY="sk-ant-your-key"
```

### Security

**Linux/macOS:**
- Set file permissions: `chmod 600 <config-file-path>`

**All platforms:**
- Never commit your **user-level** config file (it may contain API keys)
- `.gcop/config.toml` is intended for team-shared non-secret settings and can be committed
- Do not put `api_key` in project-level config; use user-level config or environment variables instead

## CI Mode

For CI/CD environments, gcop-rs provides a simplified configuration via environment variables. When `CI=1` is set, you can configure the provider using `GCOP_CI_*` variables instead of a config file.

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `CI` | Enable CI mode | `1` |
| `GCOP_CI_PROVIDER` | Provider type | `claude`, `openai`, `ollama`, or `gemini` |
| `GCOP_CI_API_KEY` | API key | `sk-ant-...` |

### Optional Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `GCOP_CI_MODEL` | Model name | `claude-sonnet-4-5-20250929` (claude)<br>`gpt-4o-mini` (openai)<br>`llama3.2` (ollama)<br>`gemini-3-flash-preview` (gemini) |
| `GCOP_CI_ENDPOINT` | Custom API endpoint | Provider default |

### Example

```bash
#!/bin/bash
# CI workflow example

export CI=1
export GCOP_CI_PROVIDER=claude
export GCOP_CI_API_KEY="$SECRET_API_KEY"  # from CI secrets
export GCOP_CI_MODEL="claude-sonnet-4-5-20250929"

# Generate commit message
gcop-rs commit --yes
```

**Benefits of CI Mode:**
- No config file needed - all configuration via environment variables
- Provider name is automatically set to "ci"
- Simplifies GitHub Actions / GitLab CI integration
- Secrets can be injected via CI/CD secret management

## Environment Overrides (GCOP__*)

In addition to CI-mode provider env vars, gcop-rs supports overriding configuration values via environment variables with the `GCOP__` prefix.

- **Priority**: `GCOP__*` overrides config file and defaults.
- **Mapping**: Nested keys are separated by **double underscores** (`__`).
- **Note**: If `CI=1`, CI-mode provider settings are applied after this stage and become the effective default provider.

**Examples**:

```bash
# Disable colors and streaming output
export GCOP__UI__COLORED=false
export GCOP__UI__STREAMING=false

# Switch default provider
export GCOP__LLM__DEFAULT_PROVIDER=openai

# Force UI language
export GCOP__UI__LANGUAGE=zh-CN
```

### Locale Selection Priority

gcop-rs resolves UI language in this order:

1. `GCOP__UI__LANGUAGE` environment variable
2. `[ui].language` in config file
3. System locale
4. Fallback to English (`en`)

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
- [Provider Health Checks](provider-health.md) - Validation behavior and health endpoints
- [Custom Prompts](prompts.md) - Customize AI prompts
- [Troubleshooting](troubleshooting.md) - Common configuration issues
