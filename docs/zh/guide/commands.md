# 命令参考

该页面现在是**导航页**。详细命令文档已经按主题拆分，以避免单页过长、便于查找。

## 全局选项

这些选项可以用于任何命令：

| 选项 | 说明 |
|------|------|
| `--provider <NAME>`, `-p` | 为 LLM 命令（`commit` / `review`）覆盖默认 provider |
| `--verbose`, `-v` | 启用详细日志（显示 API 请求和响应） |
| `--help`, `-h` | 显示帮助信息 |
| `--version`, `-V` | 显示版本信息 |

**示例**:
```bash
gcop-rs --provider openai commit
gcop-rs -v review changes
```

## 命令页

| 命令 | 使用场景 | 详细页面 |
|------|----------|----------|
| `init` | 首次初始化配置 | [init](./commands/init.md) |
| `commit` | 生成提交信息并提交 | [commit](./commands/commit.md) |
| `review` | 审查变更/提交/范围/文件 | [review](./commands/review.md) |
| `config` | 编辑并验证配置 | [config](./commands/config.md) |
| `alias` | 安装/列出/删除 git 别名 | [alias](./commands/alias.md) |
| `stats` | 查看仓库提交统计 | [stats](./commands/stats.md) |
| `hook` | 安装/卸载 `prepare-commit-msg` hook | [hook](./commands/hook.md) |

## 自动化与环境

- [自动化、退出码与环境变量](./commands/automation.md)

## 参考

- [Git 别名指南](aliases.md) - Git 别名详细指南
- [配置参考](configuration.md) - 所有配置选项
- [Provider 设置](providers.md) - 配置 LLM providers
- [Provider 健康检查](provider-health.md) - `config validate` 的检查机制
- [故障排除](troubleshooting.md) - 常见问题
