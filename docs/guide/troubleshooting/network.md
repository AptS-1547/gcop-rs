# Network Issues

## Issue: "API request timeout"

**Cause**: Request took longer than 120 seconds

**Solution**:
1. Check your internet connection
2. Try again (may be temporary server slowness)
3. If using proxy, verify proxy is working:
   ```bash
   curl -x $HTTP_PROXY https://api.openai.com
   ```
4. Increase `network.request_timeout` in config if needed, then retry manually

> **Note**: Timeout errors currently fail fast (no automatic retry). Auto-retry applies to connection failures and HTTP 429 rate limits.

## Issue: "API connection failed"

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
   # DEBUG OpenAI API network error (attempt 1/4): ... Retrying in 1.0s...
   # DEBUG OpenAI API network error (attempt 2/4): ... Retrying in 2.0s...
   ```

**Note**: Connection failures automatically retry with exponential backoff (1s, 2s, 4s).

## Issue: "Network behind proxy"

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

## Issue: Rate limit despite auto-retry

**Cause**: 429 errors persist even after retries

**Solution**:
1. **Wait longer** - The retry mechanism uses exponential backoff, but you may need to wait several minutes
2. **Check your API usage** on the provider's dashboard
3. **Upgrade your plan** if you're on a free tier
4. **Use different provider temporarily**:
   ```bash
   gcop-rs --provider claude commit  # Switch providers
   ```

## Understanding Auto-Retry

gcop-rs automatically retries specific failures:

**What gets retried**:
- ✅ Connection failures
- ✅ HTTP 429 rate limit errors
- ❌ Request timeout errors
- ❌ Other HTTP errors (401/403/400/5xx)

**Retry strategy**:
- Maximum 3 retries (4 attempts total)
- Exponential backoff: 1s → 2s → 4s
- Visible in verbose mode (`-v`)

**Example retry log**:
```
DEBUG OpenAI API request failed [connection failed]: ...
DEBUG OpenAI API network error (attempt 1/4): ... Retrying in 1.0s...
DEBUG OpenAI API network error (attempt 2/4): ... Retrying in 2.0s...
DEBUG OpenAI API request succeeded after 3 attempts
```

## Issue: "Failed to parse Claude/OpenAI response"

**Cause**: Unexpected API response format

**Solution**:
```bash
# Use verbose mode to see raw response
gcop-rs -v commit

# Check the response in debug output
# Look for "Claude API response body:" or "OpenAI API response body:"
```
