# 配置指南

## 配置文件位置

gcop-rs 使用 TOML 配置文件，位置因平台而异：

| 平台 | 位置 |
|------|------|
| Linux | `~/.config/gcop/config.toml` |
| macOS | `~/Library/Application Support/gcop/config.toml` |
| Windows | `%APPDATA%\gcop\config\config.toml` |

配置文件是**可选的**。如果不存在，将使用默认值。

## 快速设置

**推荐：使用 init 命令**

```bash
gcop-rs init
```

这将在正确的平台特定位置创建配置文件。

**手动设置：**

Linux:
```bash
mkdir -p ~/.config/gcop
cp examples/config.toml.example ~/.config/gcop/config.toml
```

macOS:
```bash
mkdir -p ~/Library/Application\ Support/gcop
cp examples/config.toml.example ~/Library/Application\ Support/gcop/config.toml
```

Windows (PowerShell):
```powershell
New-Item -ItemType Directory -Force -Path "$env:APPDATA\gcop\config"
Copy-Item examples\config.toml.example "$env:APPDATA\gcop\config\config.toml"
```

然后编辑配置文件添加你的 API key。

## 基础配置

使用 Claude API 的最小配置：

```toml
[llm]
default_provider = "claude"

[llm.providers.claude]
api_key = "sk-ant-your-key-here"
model = "claude-sonnet-4-5-20250929"
```

## 完整配置示例

```toml
# LLM 配置
[llm]
default_provider = "claude"
# fallback_providers = ["openai", "ollama"]  # 主 provider 失败时自动切换

# Claude Provider
[llm.providers.claude]
api_key = "sk-ant-your-key"
endpoint = "https://api.anthropic.com/v1/messages"
model = "claude-sonnet-4-5-20250929"
temperature = 0.3
max_tokens = 2000

# OpenAI Provider
[llm.providers.openai]
api_key = "sk-your-openai-key"
endpoint = "https://api.openai.com/v1/chat/completions"
model = "gpt-4-turbo"
temperature = 0.3

# Ollama Provider（本地）
[llm.providers.ollama]
endpoint = "http://localhost:11434/api/generate"
model = "codellama:13b"

# Commit 行为
[commit]
show_diff_preview = true
allow_edit = true
confirm_before_commit = true  # 预留字段（当前未生效）
max_retries = 10

# Review 设置
[review]
show_full_diff = true  # 预留字段（当前未生效）
min_severity = "info"  # critical | warning | info

# UI 设置
[ui]
colored = true
verbose = false
streaming = true  # 启用流式输出（实时打字效果）
language = "en"  # 可选：强制 UI 语言（如 "en"、"zh-CN"）

# 注意：流式输出支持 OpenAI 与 Claude 风格的 API。
# Ollama 会自动回退到转圈圈模式。

# 网络设置
[network]
request_timeout = 120    # HTTP 请求超时（秒）
connect_timeout = 10     # HTTP 连接超时（秒）
max_retries = 3          # API 请求失败时的最大重试次数
retry_delay_ms = 1000    # 初始重试延迟（毫秒，指数退避）
max_retry_delay_ms = 60000  # 最大重试延迟，也作为 Retry-After 头的上限

# 文件设置
[file]
max_size = 10485760      # 最大文件大小（10MB）
```

## 配置选项

### LLM 设置

| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `default_provider` | String | `"claude"` | 默认使用的 LLM provider |
| `fallback_providers` | Array | `[]` | 备用 provider 列表，主 provider 失败时自动切换 |

### Provider 设置

每个 `[llm.providers.<name>]` 下的 provider 支持：

| 选项 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `api_style` | String | 否 | API 风格：`"claude"`、`"openai"` 或 `"ollama"`（未设置时默认使用 provider 名称） |
| `api_key` | String | 是* | API key（*Ollama 不需要） |
| `endpoint` | String | 否 | API 端点（未设置时使用默认值） |
| `model` | String | 是 | 模型名称 |
| `temperature` | Float | 否 | 温度参数（0.0-1.0）。Claude/OpenAI 风格默认 0.3；Ollama 未设置时使用模型默认值 |
| `max_tokens` | Integer | 否 | 最大响应 token 数。Claude 风格默认 2000；OpenAI 风格仅在设置时发送；Ollama 当前会忽略该字段 |

### Commit 设置

| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `show_diff_preview` | Boolean | `true` | 生成前显示 diff 统计 |
| `allow_edit` | Boolean | `true` | 允许编辑生成的消息 |
| `confirm_before_commit` | Boolean | `true` | *(当前未使用)* 预留给未来“提交前最终确认”能力 |
| `max_retries` | Integer | `10` | 重新生成的最大次数 |
| `custom_prompt` | String | 无 | 自定义 system prompt / 指令（用于提交信息生成） |

### Review 设置

| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `show_full_diff` | Boolean | `true` | *(当前未使用)* 预留给未来“完整/精简 diff”切换能力 |
| `min_severity` | String | `"info"` | 最低显示的严重性：`"critical"`、`"warning"` 或 `"info"` |
| `custom_prompt` | String | 无 | 自定义 system prompt / 指令（用于代码审查） |

### UI 设置

| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `colored` | Boolean | `true` | 启用彩色输出 |
| `verbose` | Boolean | `false` | *(目前未使用)* 请使用 `--verbose` / `-v` 启用 debug 日志 |
| `streaming` | Boolean | `true` | 启用流式输出（实时打字效果） |
| `language` | String | `null`（自动） | 强制 UI 语言（如 `"en"`、`"zh-CN"`）；未设置时自动检测 |

> **关于流式输出：** 目前仅 OpenAI 和 Claude 风格的 API 支持流式输出。使用 Ollama 时，系统会自动回退到转圈圈模式（等待完整响应）。

### 网络设置

| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `request_timeout` | Integer | `120` | HTTP 请求超时（秒） |
| `connect_timeout` | Integer | `10` | HTTP 连接超时（秒） |
| `max_retries` | Integer | `3` | API 请求失败时的最大重试次数 |
| `retry_delay_ms` | Integer | `1000` | 初始重试延迟（毫秒，指数退避） |
| `max_retry_delay_ms` | Integer | `60000` | 最大重试延迟（毫秒），也作为 Retry-After 头的上限 |

### 文件设置

| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `max_size` | Integer | `10485760` | 文件审查的最大文件大小（字节，默认: 10MB） |

## API Key 配置

### 优先级顺序

1. **配置文件**（平台特定位置，见上方）
2. **环境变量**（fallback）

### 配置方式

**方式 1: 配置文件（推荐）**

```toml
[llm.providers.claude]
api_key = "sk-ant-your-key"
```

**方式 2: 环境变量**

```bash
export ANTHROPIC_API_KEY="sk-ant-your-key"
export OPENAI_API_KEY="sk-your-openai-key"
```

### 安全建议

**Linux/macOS:**
- 设置文件权限: `chmod 600 <配置文件路径>`

**所有平台:**
- 不要将 config.toml 提交到 git
- 如果创建项目级配置，添加到 .gitignore

## 环境变量覆盖（GCOP_*）

除了 provider API key 的环境变量外，gcop-rs 也支持用 `GCOP_` 前缀的环境变量覆盖配置项。

- **优先级**：`GCOP_*` 的优先级高于配置文件与默认值。
- **映射方式**：嵌套配置项使用下划线分隔。

**示例**：

```bash
# 关闭彩色与流式输出
export GCOP_UI_COLORED=false
export GCOP_UI_STREAMING=false

# 切换默认 provider
export GCOP_LLM_DEFAULT_PROVIDER=openai

# 强制 UI 语言
export GCOP_UI_LANGUAGE=zh-CN
```

### 语言选择优先级

gcop-rs 会按以下顺序决定 UI 语言：

1. 环境变量 `GCOP_UI_LANGUAGE`
2. 配置文件中的 `[ui].language`
3. 系统语言
4. 回退到英文（`en`）

## 命令行覆盖

```bash
# 覆盖 provider
gcop-rs --provider openai commit

# 启用详细模式
gcop-rs -v commit
```

命令行选项优先级高于配置文件。

## 参考

- [Provider 设置](providers.md) - 配置 LLM 提供商
- [Provider 健康检查](provider-health.md) - 验证机制与健康检查端点
- [自定义 Prompt](prompts.md) - 自定义 AI prompts
- [故障排除](troubleshooting.md) - 常见配置问题
