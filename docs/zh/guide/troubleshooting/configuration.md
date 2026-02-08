# 配置问题

## 问题: "Provider 'xxx' not found in config"

**原因**: Provider 未在平台对应的 `config.toml` 中配置

**解决方案**:
```bash
# 安全打开并编辑配置（推荐）
gcop-rs config edit

# 或手动编辑配置文件路径
# 见: /zh/guide/configuration#配置文件位置
```

## 问题: "API key not found"

**原因**: provider 配置中没有 API key（或 CI 模式变量未设置）

**解决方案**:

**选项 1**: 添加到配置文件
```toml
[llm.providers.claude]
api_key = "sk-ant-your-key"
```

**选项 2**: 使用 CI 模式环境变量
```bash
export CI=1
export GCOP_CI_PROVIDER=claude
export GCOP_CI_API_KEY="sk-ant-your-key"
```

## 问题: "Unsupported api_style"

**原因**: 配置中的 `api_style` 值无效

**解决方案**: 使用支持的值之一：
- `"claude"` - 用于 Anthropic API 兼容服务
- `"openai"` - 用于 OpenAI API 兼容服务
- `"ollama"` - 用于本地 Ollama
- `"gemini"` - 用于兼容 Google Gemini GenerateContent API 的服务
