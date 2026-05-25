# Provider 配置

gcop-rs 支持多个 LLM provider。你可以使用内置 provider 或添加自定义 provider。

`gcop-rs` 不会内置模型白名单；下面列出的模型只是常见示例，不代表穷尽列表或内建校验列表。

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

**示例模型**：
- `claude-sonnet-4-5-20250929`（推荐）
- `claude-opus-4-5-20251101`（最强大）
- `claude-3-5-sonnet-20241022`（旧版）

### OpenAI

```toml
[llm.providers.openai]
api_style = "openai"  # 内置 provider 可省略；会从名称推断
api_key = "sk-your-openai-key"
model = "gpt-4o-mini"
temperature = 0.3
```

**获取 API Key**: https://platform.openai.com/

对于需要或更适合 Responses API 的 OpenAI 模型，使用 `api_style = "openai-response"`：

```toml
[llm.providers.openai-response]
api_style = "openai-response"
api_key = "sk-your-openai-key"
endpoint = "https://api.openai.com"
model = "gpt-4o-mini"
```

该模式下 gcop-rs 会把 system prompt 作为 `instructions` 发送，把 user prompt 作为 `input` 发送，将 `max_tokens` 映射为 `max_output_tokens`，通过 `tool_choice = "none"` 禁用工具调用，并解析响应中的 `output_text` 内容。

**示例模型**：
- `gpt-4o-mini`（对应内置 CI 默认值）
- `gpt-4o`
- 任意兼容 Chat Completions 或 Responses API 的 OpenAI 模型

### Ollama（本地）

```toml
[llm.providers.ollama]
endpoint = "http://localhost:11434"
model = "llama3.2"
```

**设置**：
```bash
# 安装 Ollama
curl https://ollama.ai/install.sh | sh

# 拉取模型
ollama pull llama3.2

# 启动服务
ollama serve
```

**示例模型**: Ollama 中的任意模型（如 `llama3.2`、`qwen2.5-coder`、`deepseek-coder-v2` 等）

### Gemini（Google）

```toml
[llm.providers.gemini]
api_key = "AIza-your-gemini-key"
model = "gemini-3-flash-preview"
temperature = 0.3
```

**获取 API Key**: https://ai.google.dev/

**示例模型**：
- `gemini-3-flash-preview`（推荐默认）
- `gemini-2.5-flash`
- `gemini-2.5-pro`

## 自定义 Providers

你可以使用 `api_style` 参数添加 OpenAI、Claude 或 Gemini 兼容的 API。

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
| `"openai-response"` | OpenAI Responses API | OpenAI Responses API |
| `"claude"` | Anthropic Messages API | Claude、Claude 代理/镜像 |
| `"ollama"` | Ollama Generate API | 仅本地 Ollama |
| `"gemini"` | Google Gemini GenerateContent API | Gemini 以及兼容 Gemini 的端点 |

如果未指定 `api_style`，默认使用 provider 名称（用于向后兼容内置 providers）。

## Endpoint 规则

- Claude、OpenAI 和 Ollama 的 `endpoint` 可以填写基础 URL，也可以直接填写完整请求路径。
- 使用 OpenAI Responses API 时设置 `api_style = "openai-response"`；`endpoint = "https://api.openai.com"` 这样的基础 URL 会自动拼接为 `/v1/responses`。
- Gemini 的 `endpoint` 需要填写基础 URL；gcop-rs 会基于这个基础 URL 自动拼出 `/v1beta/models/{model}:generateContent`。

## Thinking 标签

部分 OpenAI 兼容模型会把可见推理内容放在 `<thinking>...</thinking>` 或 `<think>...</think>` 块中。gcop-rs 默认保留这些内容。要在生成 commit message 以及解析 review JSON 前移除它们，可以启用：

```toml
[llm.providers.openai]
strip_thinking = true
```

## 切换 Providers

### 使用命令行

```bash
# 为单个命令使用不同的 provider
gcop-rs --provider openai commit
gcop-rs --provider deepseek review changes
```

### 修改默认值

编辑平台对应的配置文件（见[配置指南](configuration.md#配置文件层级)）：

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

- `GCOP_CI_PROVIDER` - Provider 类型：`claude`、`openai`、`ollama` 或 `gemini`
- `GCOP_CI_API_KEY` - API key
- `GCOP_CI_MODEL`（可选，有默认值）
- `GCOP_CI_ENDPOINT`（可选）

## 参考

- [配置参考](configuration.md) - 所有配置选项
- [Provider 健康检查](provider-health.md) - `gcop-rs config validate` 的检查机制
- [自定义 Prompt](prompts.md) - 自定义 AI 行为
- [故障排除](troubleshooting.md) - Provider 连接问题
