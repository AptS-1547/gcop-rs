# Provider 配置

gcop-rs 支持多个 LLM provider。你可以使用内置 provider 或添加自定义 provider。

## 内置 Providers

### Claude (Anthropic)

```toml
[llm.providers.claude]
api_key = "sk-ant-your-key"
model = "claude-sonnet-4-5-20250929"
temperature = 0.3
max_tokens = 2000
```

**获取 API Key**: https://console.anthropic.com/

**可用模型**：
- `claude-sonnet-4-5-20250929`（推荐）
- `claude-opus-4-5-20251101`（最强大）
- `claude-3-5-sonnet-20241022`（旧版）

### OpenAI

```toml
[llm.providers.openai]
api_key = "sk-your-openai-key"
model = "gpt-4-turbo"
temperature = 0.3
```

**获取 API Key**: https://platform.openai.com/

**可用模型**：
- `gpt-4-turbo`
- `gpt-4`
- `gpt-3.5-turbo`

### Ollama（本地）

```toml
[llm.providers.ollama]
endpoint = "http://localhost:11434/api/generate"
model = "codellama:13b"
```

**设置**：
```bash
# 安装 Ollama
curl https://ollama.ai/install.sh | sh

# 拉取模型
ollama pull codellama:13b

# 启动服务
ollama serve
```

**可用模型**: Ollama 中的任意模型（codellama、llama2、mistral 等）

## 自定义 Providers

你可以使用 `api_style` 参数添加任意 OpenAI 或 Claude 兼容的 API。

### DeepSeek

```toml
[llm.providers.deepseek]
api_style = "openai"
api_key = "sk-your-deepseek-key"
endpoint = "https://api.deepseek.com/v1/chat/completions"
model = "deepseek-chat"
temperature = 0.3
```

**获取 API Key**: https://platform.deepseek.com/

### 通义千问

```toml
[llm.providers.qwen]
api_style = "openai"
api_key = "sk-your-qwen-key"
endpoint = "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"
model = "qwen-max"
```

### Claude 代理/镜像

```toml
[llm.providers.claude-code-hub]
api_style = "claude"
api_key = "your-key"
endpoint = "https://your-claude-code-hub.com/v1/messages"
model = "claude-sonnet-4-5-20250929"
```

### 自定义 OpenAI 兼容服务

```toml
[llm.providers.my-llm]
api_style = "openai"
api_key = "your-key"
endpoint = "https://api.example.com/v1/chat/completions"
model = "custom-model"
```

## API Style 参数

`api_style` 参数决定使用哪种 API 实现：

| 值 | 说明 | 兼容服务 |
|----|------|----------|
| `"openai"` | OpenAI Chat Completions API | OpenAI、DeepSeek、通义千问、大多数自定义服务 |
| `"claude"` | Anthropic Messages API | Claude、Claude 代理/镜像 |
| `"ollama"` | Ollama Generate API | 仅本地 Ollama |

如果未指定 `api_style`，默认使用 provider 名称（用于向后兼容内置 providers）。

## 切换 Providers

### 使用命令行

```bash
# 为单个命令使用不同的 provider
gcop-rs --provider openai commit
gcop-rs --provider deepseek review changes
```

### 修改默认值

编辑 `~/.config/gcop/config.toml`：

```toml
[llm]
default_provider = "deepseek"  # 修改这里
```

## API Key 管理

### 配置文件

Provider 的 `api_key` 在 `config.toml` 中设置：

```toml
[llm.providers.claude]
api_key = "sk-ant-..."
```

### CI 模式环境变量

在 CI 模式（`CI=1`）下，使用环境变量代替配置文件：

- `GCOP_CI_PROVIDER` - Provider 类型：`claude`、`openai` 或 `ollama`
- `GCOP_CI_API_KEY` - API key
- `GCOP_CI_MODEL`（可选，有默认值）
- `GCOP_CI_ENDPOINT`（可选）

## 参考

- [配置参考](configuration.md) - 所有配置选项
- [Provider 健康检查](provider-health.md) - `gcop-rs config validate` 的检查机制
- [自定义 Prompt](prompts.md) - 自定义 AI 行为
- [故障排除](troubleshooting.md) - Provider 连接问题
