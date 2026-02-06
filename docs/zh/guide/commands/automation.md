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

| 变量 | 说明 |
|------|------|
| `ANTHROPIC_API_KEY` | Claude API key（如果不在配置中则作为回退） |
| `OPENAI_API_KEY` | OpenAI API key（回退） |
| `VISUAL` / `EDITOR` | commit message 编辑与 `gcop-rs config edit` 使用的编辑器 |
| `GCOP_UI_LANGUAGE` | 在启动早期强制指定 UI 语言（在完整加载配置前生效） |
| `GCOP_*` | 通过环境变量覆盖配置项（如 `GCOP_UI_COLORED=false`） |
| `NO_COLOR` | 禁用彩色输出（设置为任意值） |

**示例**:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export EDITOR="vim"
gcop-rs commit
```

## 参考

- [Git 别名指南](../aliases.md) - Git 别名详细指南
- [配置参考](../configuration.md) - 所有配置选项
- [Provider 设置](../providers.md) - 配置 LLM providers
- [故障排除](../troubleshooting.md) - 常见问题
