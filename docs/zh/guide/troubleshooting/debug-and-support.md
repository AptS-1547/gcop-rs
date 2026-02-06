# 调试模式

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
2. 查看[配置参考](../configuration.md)
3. 查看 [Provider 设置指南](../providers.md)
4. 在 GitHub 上开 issue，包括：
   - 你的配置文件（删除 API keys！）
   - 运行的命令
   - 错误信息
   - `gcop-rs -v` 的输出（删除敏感信息）

