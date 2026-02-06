# 故障排除

gcop-rs 的常见问题和解决方案。

## 安装问题

### 问题: `cargo build` 失败

**解决方案**:
```bash
# 更新 Rust
rustup update

# 清理并重新编译
cargo clean
cargo build --release
```

### 问题: 安装后找不到二进制文件

**解决方案**:
```bash
# 检查二进制文件是否存在
ls -la /usr/local/bin/gcop-rs

# 验证 PATH 包含 /usr/local/bin
echo $PATH

# 如需要添加到 PATH
export PATH="/usr/local/bin:$PATH"
```

## 配置问题

### 问题: "Provider 'xxx' not found in config"

**原因**: Provider 未在 `~/.config/gcop/config.toml` 中配置

**解决方案**:
```bash
# 检查配置文件
cat ~/.config/gcop/config.toml

# 复制示例配置
cp examples/config.toml.example ~/.config/gcop/config.toml

# 编辑并添加 provider
vim ~/.config/gcop/config.toml
```

### 问题: "API key not found"

**原因**: 配置文件和环境变量中都没有 API key

**解决方案**:

**选项 1**: 添加到配置文件
```toml
[llm.providers.claude]
api_key = "sk-ant-your-key"
```

**选项 2**: 使用环境变量
```bash
export ANTHROPIC_API_KEY="sk-ant-your-key"
```

### 问题: "Unsupported api_style"

**原因**: 配置中的 `api_style` 值无效

**解决方案**: 使用支持的值之一：
- `"claude"` - 用于 Anthropic API 兼容服务
- `"openai"` - 用于 OpenAI API 兼容服务
- `"ollama"` - 用于本地 Ollama

## API 问题

### 问题: "401 Unauthorized"

**原因**: API key 无效或已过期

**解决方案**:
1. 验证 API key 是否正确
2. 检查 key 是否过期
3. 从 provider 控制台重新生成 key
4. 更新 config.toml 中的新 key

### 问题: "429 Rate Limit Exceeded"

**原因**: 请求过多

**解决方案**:
1. 稍等片刻再重试
2. 升级你的 API 计划
3. 临时切换到其他 provider

### 问题: "500 Internal Server Error"

**原因**: API 服务暂时不可用

**解决方案**:
1. 等待并重试
2. 检查 provider 的状态页面
3. 尝试其他 provider

## 网络问题

### 问题: "API request timeout"

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

### 问题: "API connection failed"

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

### 问题: "网络需要代理"

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

### 问题: 即使自动重试仍然遇到限流

**原因**: 429 错误在重试后依然存在

**解决方案**:
1. **等待更长时间** - 重试机制使用指数退避，但你可能需要等待几分钟
2. **检查 API 使用情况**，在 provider 控制台查看
3. **升级套餐**，如果你在免费层
4. **临时切换 provider**：
   ```bash
   gcop-rs --provider claude commit  # 切换 provider
   ```

### 理解自动重试

从 v0.1.6 开始，gcop-rs 会自动重试失败的请求：

**会被重试的错误**：
- ✅ 连接失败
- ✅ 429 限流错误
- ❌ 请求超时错误
- ❌ 401/403 认证错误（不重试）
- ❌ 400 请求格式错误（不重试）

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

### 问题: "Failed to parse Claude/OpenAI response"

**原因**: API 响应格式异常

**解决方案**:
```bash
# 使用详细模式查看原始响应
gcop-rs -v commit

# 在调试输出中查找
# 查找 "Claude API response body:" 或 "OpenAI API response body:"
```

## 代码审查问题

### 问题: "Failed to parse review result"

**原因**: LLM 没有返回有效的 JSON

**解决方案**:

1. **使用详细模式**查看原始响应：
   ```bash
   gcop-rs -v review changes
   ```

2. **检查自定义 prompt**（如果使用）：
   - 确保明确要求 JSON 格式
   - 提供准确的 JSON schema 示例

3. **尝试不同模型**：
   ```bash
   # 某些模型处理 JSON 更好
   gcop-rs --provider openai review changes
   ```

4. **调整 temperature**：
   ```toml
   temperature = 0.1  # 更低 = 更一致的输出
   ```

## Git 问题

### 问题: "No staged changes found"

**原因**: Git 暂存区为空

**解决方案**:
```bash
# 先暂存变更
git add <files>

# 或暂存所有变更
git add .

# 然后运行 gcop
gcop-rs commit
```

### 问题: "Not a git repository"

**原因**: 当前目录不是 git 仓库

**解决方案**:
```bash
# 初始化 git 仓库
git init

# 或在 git 仓库中运行 gcop
cd /path/to/your/git/repo
```

## 调试模式

对于任何问题，启用详细模式获取详细信息：

```bash
gcop-rs -v commit
gcop-rs -v review changes
```

这会显示：
- 配置加载过程
- API 请求和响应
- 运行 `gcop-rs -v commit` 时的 commit prompt（system + user）
- 响应解析过程

> **安全提示**: verbose 模式（`-v` 或 `RUST_LOG=debug`）会在日志中打印完整的 API 请求和响应，可能包含：
> - 你的代码 diff 和变更内容
> - 错误信息中的部分 API key
> - 生成的 commit message
>
> 不要公开分享这些日志或将其提交到版本控制中。

## 获取帮助

如果遇到这里未列出的问题：

1. 使用 `--verbose` 运行并检查日志
2. 查看[配置参考](configuration.md)
3. 查看 [Provider 设置指南](providers.md)
4. 在 GitHub 上开 issue，包括：
   - 你的配置文件（删除 API keys！）
   - 运行的命令
   - 错误信息
   - `gcop-rs -v` 的输出（删除敏感信息）

## 参考

- [配置参考](configuration.md)
- [Provider 设置](providers.md)
- [自定义 Prompt](prompts.md)
