# 自动化、退出码与环境变量

## 命令链接

gcop-rs 命令可以与标准 git 命令组合：

```bash
# 先审查，再暂存并提交
gcop-rs review changes && git add -A && gcop-rs commit

# 提交后推送（使用完整命令）
gcop-rs commit --yes && git push

# 或使用别名
git acp  # 等同于: add -A && commit && push
```

## 退出码

gcop-rs 使用标准退出码：

| 代码 | 含义 |
|------|------|
| 0 | 成功（在交互式菜单中取消也会返回 0） |
| 1 | 运行时错误（API 错误、git 错误、配置错误等） |
| 2 | 命令行用法错误（参数/选项无效，由 clap 返回） |

**在脚本中使用**:
```bash
if gcop-rs commit --yes; then
    echo "提交成功"
    git push
else
    echo "提交失败"
fi
```

## 环境变量

这些环境变量会影响 gcop-rs 行为：

### CI 模式（简化配置）

| 变量 | 说明 |
|------|------|
| `CI=1` | 启用 CI 模式，使用简化的 provider 配置 |
| `GCOP_CI_PROVIDER` | Provider 类型：`claude`、`openai`、`ollama` 或 `gemini`（CI 模式必需） |
| `GCOP_CI_API_KEY` | Provider 的 API key（CI 模式必需） |
| `GCOP_CI_MODEL` | 模型名称（可选，有默认值） |
| `GCOP_CI_ENDPOINT` | 自定义 API 端点（可选） |

**CI 模式示例**:
```bash
export CI=1
export GCOP_CI_PROVIDER=claude
export GCOP_CI_API_KEY="sk-ant-..."
export GCOP_CI_MODEL="claude-sonnet-4-5-20250929"  # 可选
gcop-rs commit
```

### 通用配置

| 变量 | 说明 |
|------|------|
| `GCOP__*` | 覆盖配置项（嵌套层级使用双下划线，如 `GCOP__UI__COLORED=false`） |
| `GCOP__UI__LANGUAGE` | 在启动早期强制指定 UI 语言（使用双下划线，与其他嵌套键一致） |
| `VISUAL` / `EDITOR` | commit message 编辑与 `gcop-rs config edit` 使用的编辑器 |

**配置覆盖示例**:
```bash
export GCOP__UI__COLORED=false
export GCOP__LLM__DEFAULT_PROVIDER=openai
export EDITOR="vim"
gcop-rs commit
```

## 参考

- [Git 别名指南](../aliases.md) - Git 别名详细指南
- [配置参考](../configuration.md) - 所有配置选项
- [Provider 设置](../providers.md) - 配置 LLM providers
- [故障排除](../troubleshooting.md) - 常见问题
