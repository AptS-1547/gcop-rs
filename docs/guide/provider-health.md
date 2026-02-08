# Provider Health Checks (`gcop-rs config validate`)

This page explains what `gcop-rs config validate` actually checks and how to troubleshoot failures.

## Quick Start

```bash
gcop-rs config validate
```

The command runs in two steps:

1. Load and parse configuration
2. Validate provider connectivity

## What Is Validated

`config validate` uses your configured provider chain:

- Primary provider: `[llm].default_provider`
- Optional fallbacks: `[llm].fallback_providers`

Validation succeeds if **at least one provider** in that chain validates successfully.

If a configured provider cannot be instantiated (for example, missing/invalid config), it is skipped during chain construction.

## Provider-Specific Validation Behavior

### Claude / OpenAI / Gemini Style Providers

For Claude, OpenAI, and Gemini style providers, gcop-rs sends a minimal test request:

- A small prompt (`"test"`)
- `max_tokens = 1` (minimal token cost)
- Directly to the configured `endpoint`

This confirms:

- Endpoint is reachable
- API key is accepted
- Model and request shape are valid enough for a real call

### Ollama Provider

For Ollama, gcop-rs performs a health-style check on:

- `.../api/tags` (derived from your configured Ollama generate endpoint)

Then it verifies the configured model exists in the returned model list.

## Common Failure Patterns

### Endpoint Connection Failure

Symptoms:

- Connection errors
- Timeout errors

Checks:

- Verify `endpoint` URL spelling and scheme (`http` / `https`)
- Confirm network/proxy settings
- For Ollama, ensure local service is running (`ollama serve`)

### Authentication Failure (401/403)

Checks:

- Confirm API key is valid and not expired
- Confirm provider key is set in config or environment variable
- Check provider/account permissions

### Rate Limit / Server Errors (429/5xx)

Checks:

- Retry later
- Test with a backup provider via `fallback_providers`

## Suggested Configuration Pattern

```toml
[llm]
default_provider = "claude"
fallback_providers = ["openai", "gemini", "ollama"]

[llm.providers.gemini]
api_style = "gemini"
api_key = "AIza..."
model = "gemini-3-flash-preview"

[llm.providers.claude]
api_style = "claude"
api_key = "sk-ant-..."
model = "claude-3-5-haiku-20241022"

[llm.providers.openai]
api_style = "openai"
api_key = "sk-..."
model = "gpt-4o-mini"

[llm.providers.ollama]
api_style = "ollama"
endpoint = "http://localhost:11434/api/generate"
model = "llama3"
```

## See Also

- [Configuration Guide](configuration.md)
- [LLM Providers](providers.md)
- [Troubleshooting](troubleshooting.md)
