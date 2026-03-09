# Provider Configuration

gcop-rs supports multiple LLM providers. You can use built-in providers or add custom ones.

`gcop-rs` does not hardcode model allowlists. The examples below are common choices, not an exhaustive or validated list.

## Built-in Providers

### Claude (Anthropic)

```toml
[llm.providers.claude]
api_key = "sk-ant-your-key"
model = "claude-sonnet-4-5-20250929"
temperature = 0.3
max_tokens = 2000
```

**Get API Key**: https://console.anthropic.com/

**Example Models**:
- `claude-sonnet-4-5-20250929` (recommended)
- `claude-opus-4-5-20251101` (most powerful)
- `claude-3-5-sonnet-20241022` (older version)

### OpenAI

```toml
[llm.providers.openai]
api_key = "sk-your-openai-key"
model = "gpt-4o-mini"
temperature = 0.3
```

**Get API Key**: https://platform.openai.com/

**Example Models**:
- `gpt-4o-mini` (matches the built-in CI default)
- `gpt-4o`
- Any Chat Completions compatible model from OpenAI or a compatible service

### Ollama (Local)

```toml
[llm.providers.ollama]
endpoint = "http://localhost:11434"
model = "llama3.2"
```

**Setup**:
```bash
# Install Ollama
curl https://ollama.ai/install.sh | sh

# Pull a model
ollama pull llama3.2

# Start server
ollama serve
```

**Example Models**: Any model available in Ollama (`llama3.2`, `qwen2.5-coder`, `deepseek-coder-v2`, etc.)

### Gemini (Google)

```toml
[llm.providers.gemini]
api_key = "AIza-your-gemini-key"
model = "gemini-3-flash-preview"
temperature = 0.3
```

**Get API Key**: https://ai.google.dev/

**Example Models**:
- `gemini-3-flash-preview` (recommended default)
- `gemini-2.5-flash`
- `gemini-2.5-pro`

## Custom Providers

You can add OpenAI-, Claude-, or Gemini-compatible APIs using the `api_style` parameter.

### DeepSeek

```toml
[llm.providers.deepseek]
api_style = "openai"
api_key = "sk-your-deepseek-key"
endpoint = "https://api.deepseek.com/v1/chat/completions"
model = "deepseek-chat"
temperature = 0.3
```

**Get API Key**: https://platform.deepseek.com/

### Qwen (通义千问)

```toml
[llm.providers.qwen]
api_style = "openai"
api_key = "sk-your-qwen-key"
endpoint = "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"
model = "qwen-max"
```

### Claude Proxy/Mirror

```toml
[llm.providers.claude-code-hub]
api_style = "claude"
api_key = "your-key"
endpoint = "https://your-claude-code-hub.com/v1/messages"
model = "claude-sonnet-4-5-20250929"
```

### Custom OpenAI Compatible Service

```toml
[llm.providers.my-llm]
api_style = "openai"
api_key = "your-key"
endpoint = "https://api.example.com/v1/chat/completions"
model = "custom-model"
```

## API Style Parameter

The `api_style` parameter determines which API implementation to use:

| Value | Description | Compatible Services |
|-------|-------------|-------------------|
| `"openai"` | OpenAI Chat Completions API | OpenAI, DeepSeek, Qwen, most custom services |
| `"claude"` | Anthropic Messages API | Claude, Claude proxies/mirrors |
| `"ollama"` | Ollama Generate API | Local Ollama only |
| `"gemini"` | Google Gemini GenerateContent API | Gemini and Gemini-compatible endpoints |

If `api_style` is not specified, it defaults to the provider name (for backward compatibility with built-in providers).

## Endpoint Rules

- Claude, OpenAI, and Ollama providers accept either a base URL or a full request path in `endpoint`.
- Gemini expects a base URL in `endpoint`; gcop-rs derives `/v1beta/models/{model}:generateContent` from that base.

## Switching Providers

### Using Command-Line

```bash
# Use different provider for one command
gcop-rs --provider openai commit
gcop-rs --provider deepseek review changes
```

### Changing Default

Edit your platform-specific config file (see [Configuration Guide](configuration.md#configuration-files)):

```toml
[llm]
default_provider = "deepseek"  # Change this
```

## API Key Management

### Config File

Provider `api_key` is configured in `config.toml`:

```toml
[llm.providers.claude]
api_key = "sk-ant-..."
```

### CI Mode Environment Variables

In CI mode (`CI=1`), use environment variables instead of config file:

- `GCOP_CI_PROVIDER` - Provider type: `claude`, `openai`, `ollama`, or `gemini`
- `GCOP_CI_API_KEY` - API key
- `GCOP_CI_MODEL` (optional, has defaults)
- `GCOP_CI_ENDPOINT` (optional)

## See Also

- [Configuration Reference](configuration.md) - All configuration options
- [Provider Health Checks](provider-health.md) - How `gcop-rs config validate` works
- [Custom Prompts](prompts.md) - Customize AI behavior
- [Troubleshooting](troubleshooting.md) - Provider connection issues
