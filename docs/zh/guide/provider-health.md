# Provider 健康检查（`gcop-rs config validate`）

本页说明 `gcop-rs config validate` 实际检查了什么，以及失败时如何排查。

## 快速使用

```bash
gcop-rs config validate
```

该命令分两步执行：

1. 加载并解析配置
2. 验证 provider 连通性

## 验证范围

`config validate` 会使用你配置的 provider 链：

- 主 provider：`[llm].default_provider`
- 备用 provider：`[llm].fallback_providers`（可选）

只要这个链路中**至少一个 provider** 验证成功，命令就会返回成功。

如果某个 provider 在创建阶段就失败（例如配置缺失或无效），它会在构建链路时被跳过。

## 不同 Provider 的验证方式

### Claude / OpenAI 风格 Provider

对 Claude 与 OpenAI 风格 provider，gcop-rs 会发送最小化测试请求：

- 最小 prompt（`"test"`）
- `max_tokens = 1`（尽量降低 token 开销）
- 直接请求配置中的 `endpoint`

这会验证：

- endpoint 是否可达
- API key 是否可用
- model 与请求格式是否可正常调用

### Ollama Provider

对 Ollama，gcop-rs 会进行健康检查风格请求：

- `.../api/tags`（由配置的 Ollama generate endpoint 推导）

随后还会检查配置的 `model` 是否存在于返回模型列表中。

## 常见失败场景

### Endpoint 连接失败

表现：

- 连接失败
- 请求超时

排查：

- 检查 `endpoint` 拼写与协议（`http` / `https`）
- 检查网络与代理设置
- 若是 Ollama，确认本地服务已启动（`ollama serve`）

### 鉴权失败（401/403）

排查：

- API key 是否有效、是否过期
- 配置文件或环境变量中是否正确设置 key
- Provider 账号权限是否满足请求

### 限流或服务端错误（429/5xx）

排查：

- 稍后重试
- 通过 `fallback_providers` 配置备用 provider

## 推荐配置模式

```toml
[llm]
default_provider = "claude"
fallback_providers = ["openai", "ollama"]

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

## 参考

- [配置指南](configuration.md)
- [LLM Providers](providers.md)
- [故障排除](troubleshooting.md)
