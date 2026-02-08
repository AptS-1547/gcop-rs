# 网络问题

## 问题: "API request timeout"

**原因**: 请求超过 120 秒超时

**解决方案**:
1. 检查网络连接
2. 重试（可能是服务器临时慢）
3. 如使用代理，验证代理是否工作：
   ```bash
   curl -x $HTTP_PROXY https://api.openai.com
   ```
4. 按需提高配置中的 `network.request_timeout`，然后手动重试

> **注意**：超时错误当前为快速失败（不会自动重试）。自动重试仅适用于连接失败和 HTTP 429 限流。

## 问题: "API connection failed"

**原因**: 无法建立到 API 服务器的连接

**解决方案**:
1. **检查网络连通性**：
   ```bash
   ping 8.8.8.8
   curl https://api.openai.com
   ```

2. **验证 API endpoint 正确**：
   ```toml
   [llm.providers.openai]
   endpoint = "https://api.openai.com"  # 检查拼写
   ```

3. **检查 DNS 解析**：
   ```bash
   nslookup api.openai.com
   ```

4. **启用详细模式**查看重试尝试：
   ```bash
   gcop-rs -v commit
   # 你会看到：
   # DEBUG OpenAI API network error (attempt 1/4): ... Retrying in 1.0s...
   # DEBUG OpenAI API network error (attempt 2/4): ... Retrying in 2.0s...
   ```

**注意**: 连接失败会自动重试，使用指数退避（1s, 2s, 4s）。

## 问题: "网络需要代理"

**原因**: 你的网络需要代理才能访问外部服务

**解决方案**:

**HTTP/HTTPS 代理**：
```bash
# 临时使用（当前会话）
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080
gcop-rs commit

# 永久配置（添加到 ~/.bashrc 或 ~/.zshrc）
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080
```

**SOCKS5 代理**：
```bash
export HTTP_PROXY=socks5://127.0.0.1:1080
export HTTPS_PROXY=socks5://127.0.0.1:1080
```

**带认证的代理**：
```bash
export HTTP_PROXY=http://username:password@proxy.example.com:8080
export HTTPS_PROXY=http://username:password@proxy.example.com:8080
```

**验证代理是否工作**：
```bash
gcop-rs -v commit
# 查找：
# DEBUG reqwest::connect: proxy(http://127.0.0.1:7890/) intercepts 'https://api.openai.com/'
```

**绕过特定域名的代理**：
```bash
export NO_PROXY=localhost,127.0.0.1,.local
```

## 问题: 即使自动重试仍然遇到限流

**原因**: 429 错误在重试后依然存在

**解决方案**:
1. **等待更长时间** - 重试机制使用指数退避，但你可能需要等待几分钟
2. **检查 API 使用情况**，在 provider 控制台查看
3. **升级套餐**，如果你在免费层
4. **临时切换 provider**：
   ```bash
   gcop-rs --provider claude commit  # 切换 provider
   ```

## 理解自动重试

gcop-rs 会自动重试特定类型的失败：

**会被重试的错误**：
- ✅ 连接失败
- ✅ HTTP 429 限流错误
- ❌ 请求超时错误
- ❌ 其他 HTTP 错误（401/403/400/5xx）

**重试策略**：
- 最多重试 3 次（总共 4 次尝试）
- 指数退避：1s → 2s → 4s
- 在详细模式（`-v`）下可见

**重试日志示例**：
```
DEBUG OpenAI API request failed [connection failed]: ...
DEBUG OpenAI API network error (attempt 1/4): ... Retrying in 1.0s...
DEBUG OpenAI API network error (attempt 2/4): ... Retrying in 2.0s...
DEBUG OpenAI API request succeeded after 3 attempts
```

## 问题: "Failed to parse Claude/OpenAI/Gemini response"

**原因**: API 响应格式异常

**解决方案**:
```bash
# 使用详细模式查看原始响应
gcop-rs -v commit

# 在调试输出中查找
# 查找 "Claude API response body:"、"OpenAI API response body:" 或 "Gemini API response body:"
```
