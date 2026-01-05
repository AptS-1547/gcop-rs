# Troubleshooting

Common issues and solutions for gcop-rs.

## Installation Issues

### Issue: `cargo build` fails

**Solution**:
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

### Issue: Binary not found after install

**Solution**:
```bash
# Check if binary exists
ls -la /usr/local/bin/gcop-rs

# Verify PATH includes /usr/local/bin
echo $PATH

# Add to PATH if needed
export PATH="/usr/local/bin:$PATH"
```

## Configuration Issues

### Issue: "Provider 'xxx' not found in config"

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

### Issue: "API key not found"

**Cause**: No API key in config file or environment

**Solution**:

**Option 1**: Add to config file
```toml
[llm.providers.claude]
api_key = "sk-ant-your-key"
```

**Option 2**: Use environment variable
```bash
export ANTHROPIC_API_KEY="sk-ant-your-key"
```

### Issue: "Unsupported api_style"

**Cause**: Invalid `api_style` value in config

**Solution**: Use one of the supported values:
- `"claude"` - For Anthropic API compatible services
- `"openai"` - For OpenAI API compatible services
- `"ollama"` - For local Ollama

## API Issues

### Issue: "401 Unauthorized"

**Cause**: Invalid or expired API key

**Solution**:
1. Verify your API key is correct
2. Check if the key has expired
3. Regenerate key from provider's dashboard
4. Update config.toml with new key

### Issue: "429 Rate Limit Exceeded"

**Cause**: Too many requests

**Solution**:
1. Wait a few moments before retry
2. Upgrade your API plan
3. Switch to a different provider temporarily

### Issue: "500 Internal Server Error"

**Cause**: API service temporarily unavailable

**Solution**:
1. Wait and retry
2. Check provider's status page
3. Try a different provider

## Network Issues

### Issue: "API request timeout"

**Cause**: Request took longer than 120 seconds

**Solution**:
1. Check your internet connection
2. Try again (may be temporary server slowness)
3. If using proxy, verify proxy is working:
   ```bash
   curl -x $HTTP_PROXY https://api.openai.com
   ```
4. The request will automatically retry up to 3 times with backoff

### Issue: "API connection failed"

**Cause**: Cannot establish connection to API server

**Solution**:
1. **Check network connectivity**:
   ```bash
   ping 8.8.8.8
   curl https://api.openai.com
   ```

2. **Verify API endpoint is correct**:
   ```toml
   [llm.providers.openai]
   endpoint = "https://api.openai.com"  # Check for typos
   ```

3. **Check DNS resolution**:
   ```bash
   nslookup api.openai.com
   ```

4. **Enable verbose mode** to see retry attempts:
   ```bash
   gcop-rs -v commit
   # You'll see:
   # WARN OpenAI API request failed (attempt 1/4): connection failed. Retrying in 1.0s...
   # WARN OpenAI API request failed (attempt 2/4): connection failed. Retrying in 2.0s...
   ```

**Note**: Connection failures automatically retry with exponential backoff (1s, 2s, 4s).

### Issue: "Network behind proxy"

**Cause**: Your network requires a proxy to access external services

**Solution**:

**For HTTP/HTTPS proxy**:
```bash
# Temporary (current session)
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080
gcop-rs commit

# Permanent (add to ~/.bashrc or ~/.zshrc)
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080
```

**For SOCKS5 proxy**:
```bash
export HTTP_PROXY=socks5://127.0.0.1:1080
export HTTPS_PROXY=socks5://127.0.0.1:1080
```

**With authentication**:
```bash
export HTTP_PROXY=http://username:password@proxy.example.com:8080
export HTTPS_PROXY=http://username:password@proxy.example.com:8080
```

**Verify proxy is working**:
```bash
gcop-rs -v commit
# Look for:
# DEBUG reqwest::connect: proxy(http://127.0.0.1:7890/) intercepts 'https://api.openai.com/'
```

**Bypass proxy for specific domains**:
```bash
export NO_PROXY=localhost,127.0.0.1,.local
```

### Issue: Rate limit despite auto-retry

**Cause**: 429 errors persist even after retries

**Solution**:
1. **Wait longer** - The retry mechanism uses exponential backoff, but you may need to wait several minutes
2. **Check your API usage** on the provider's dashboard
3. **Upgrade your plan** if you're on a free tier
4. **Use different provider temporarily**:
   ```bash
   gcop-rs --provider claude commit  # Switch providers
   ```

### Understanding Auto-Retry

Since v0.1.6, gcop-rs automatically retries failed requests:

**What gets retried**:
- ✅ Connection failures
- ✅ 429 Rate limit errors
- ❌ 401/403 Authentication errors (won't retry)
- ❌ 400 Bad request errors (won't retry)

**Retry strategy**:
- Maximum 3 retries (4 attempts total)
- Exponential backoff: 1s → 2s → 4s
- Visible in verbose mode (`-v`)

**Example retry log**:
```
WARN  OpenAI API request failed (attempt 1/4): connection failed. Retrying in 1.0s...
WARN  OpenAI API request failed (attempt 2/4): connection failed. Retrying in 2.0s...
INFO  OpenAI API request succeeded after 3 attempts
```

### Issue: "Failed to parse Claude/OpenAI response"

**Cause**: Unexpected API response format

**Solution**:
```bash
# Use verbose mode to see raw response
gcop-rs -v commit

# Check the response in debug output
# Look for "Claude API response body:" or "OpenAI API response body:"
```

## Code Review Issues

### Issue: "Failed to parse review result"

**Cause**: LLM didn't return valid JSON

**Solution**:

1. **Use verbose mode** to see raw response:
   ```bash
   gcop-rs -v review changes
   ```

2. **Check your custom prompt** (if using one):
   - Ensure it explicitly requests JSON format
   - Provide exact JSON schema example

3. **Try different model**:
   ```bash
   # Some models handle JSON better
   gcop-rs --provider openai review changes
   ```

4. **Adjust temperature**:
   ```toml
   temperature = 0.1  # Lower = more consistent output
   ```

## Git Issues

### Issue: "No staged changes found"

**Cause**: Nothing added to git staging area

**Solution**:
```bash
# Stage your changes first
git add <files>

# Or stage all changes
git add .

# Then run gcop
gcop-rs commit
```

### Issue: "Not a git repository"

**Cause**: Current directory is not a git repo

**Solution**:
```bash
# Initialize git repository
git init

# Or run gcop from within a git repository
cd /path/to/your/git/repo
```

## Debug Mode

For any issue, enable verbose mode to get detailed information:

```bash
gcop-rs -v commit
gcop-rs -v review changes
```

This shows:
- Configuration loading
- API requests and responses
- Prompt sent to LLM
- Response parsing

> **Security Notice**: Verbose mode (`-v` or `RUST_LOG=debug`) logs full API requests and responses, which may include:
> - Your code diffs and changes
> - Partial API keys in error messages
> - Generated commit messages
>
> Do not share verbose logs publicly or commit them to version control.

## Getting Help

If you encounter an issue not listed here:

1. Run with `--verbose` and check the logs
2. Check the [Configuration Reference](configuration.md)
3. Review the [Provider Setup Guide](providers.md)
4. Open an issue on GitHub with:
   - Your config file (remove API keys!)
   - Command you ran
   - Error message
   - Output from `gcop-rs -v` (remove sensitive info)

## See Also

- [Configuration Reference](configuration.md)
- [Provider Setup](providers.md)
- [Custom Prompts](prompts.md)
