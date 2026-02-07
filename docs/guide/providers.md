# Provider Configuration

gcop-rs supports multiple LLM providers. You can use built-in providers or add custom ones.

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

**Available Models**:
- `claude-sonnet-4-5-20250929` (recommended)
- `claude-opus-4-5-20251101` (most powerful)
- `claude-3-5-sonnet-20241022` (older version)

### OpenAI

```toml
[llm.providers.openai]
api_key = "sk-your-openai-key"
model = "gpt-4-turbo"
temperature = 0.3
```

**Get API Key**: https://platform.openai.com/

**Available Models**:
- `gpt-4-turbo`
- `gpt-4`
- `gpt-3.5-turbo`

### Ollama (Local)

```toml
[llm.providers.ollama]
endpoint = "http://localhost:11434/api/generate"
model = "codellama:13b"
```

**Setup**:
```bash
# Install Ollama
curl https://ollama.ai/install.sh | sh

# Pull a model
ollama pull codellama:13b

# Start server
ollama serve
```

**Available Models**: Any model available in Ollama (codellama, llama2, mistral, etc.)

## Custom Providers

You can add any OpenAI or Claude compatible API using the `api_style` parameter.

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

If `api_style` is not specified, it defaults to the provider name (for backward compatibility with built-in providers).

## Switching Providers

### Using Command-Line

```bash
# Use different provider for one command
gcop-rs --provider openai commit
gcop-rs --provider deepseek review changes
```

### Changing Default

Edit `~/.config/gcop/config.toml`:

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

- `GCOP_CI_PROVIDER` - Provider type: `claude`, `openai`, or `ollama`
- `GCOP_CI_API_KEY` - API key
- `GCOP_CI_MODEL` (optional, has defaults)
- `GCOP_CI_ENDPOINT` (optional)

## See Also

- [Configuration Reference](configuration.md) - All configuration options
- [Provider Health Checks](provider-health.md) - How `gcop-rs config validate` works
- [Custom Prompts](prompts.md) - Customize AI behavior
- [Troubleshooting](troubleshooting.md) - Provider connection issues
