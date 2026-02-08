# Configuration Issues

## Issue: "Provider 'xxx' not found in config"

**Cause**: Provider not configured in your platform-specific `config.toml`

**Solution**:
```bash
# Open and edit config safely (recommended)
gcop-rs config edit

# Or manually edit your config file path
# See: /guide/configuration#configuration-file-location
```

## Issue: "API key not found"

**Cause**: No API key in provider config (or CI mode variables not set)

**Solution**:

**Option 1**: Add to config file
```toml
[llm.providers.claude]
api_key = "sk-ant-your-key"
```

**Option 2**: Use CI mode environment variables
```bash
export CI=1
export GCOP_CI_PROVIDER=claude
export GCOP_CI_API_KEY="sk-ant-your-key"
```

## Issue: "Unsupported api_style"

**Cause**: Invalid `api_style` value in config

**Solution**: Use one of the supported values:
- `"claude"` - For Anthropic API compatible services
- `"openai"` - For OpenAI API compatible services
- `"ollama"` - For local Ollama
- `"gemini"` - For Google Gemini GenerateContent API compatible services
