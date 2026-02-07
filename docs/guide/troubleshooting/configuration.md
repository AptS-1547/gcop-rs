# Configuration Issues

## Issue: "Provider 'xxx' not found in config"

**Cause**: Provider not configured in `~/.config/gcop/config.toml`

**Solution**:
```bash
# Check your config file
cat ~/.config/gcop/config.toml

# Copy example config
cp examples/config.toml.example ~/.config/gcop/config.toml

# Edit and add your provider
vim ~/.config/gcop/config.toml
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
export PROVIDER_TYPE=claude
export PROVIDER_API_KEY="sk-ant-your-key"
```

## Issue: "Unsupported api_style"

**Cause**: Invalid `api_style` value in config

**Solution**: Use one of the supported values:
- `"claude"` - For Anthropic API compatible services
- `"openai"` - For OpenAI API compatible services
- `"ollama"` - For local Ollama
