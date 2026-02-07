# config

管理 gcop-rs 配置。

**语法**:
```bash
gcop-rs config [子命令]
```

不带子命令时，默认等同于 `gcop-rs config edit`。

**子命令**:

## `config edit`

在默认编辑器中打开配置文件，并在保存后校验。

**用法**:
```bash
gcop-rs config edit
```

**打开**: 使用 `$VISUAL` / `$EDITOR` 打开配置文件（未设置时使用系统默认编辑器）

**校验**: 保存后会自动校验配置（类似 `visudo`）。如果校验失败，会显示一个菜单：

```
✗ Config validation failed: TOML parse error...

? What would you like to do?
> ✎ Re-edit the config file
  ↩ Keep original config
  ⚠ Ignore errors and save anyway (dangerous)
```

**恢复**: 即使配置文件损坏，`config edit` 仍然可以运行，让你修复它。

**何时使用**: 修改 API keys、模型或自定义 prompts。

> **提示**: 建议始终使用 `gcop-rs config edit` 而不是直接编辑配置文件，以便自动校验。

## `config validate`

验证配置并测试 provider 连接。

**用法**:
```bash
gcop-rs config validate
```

**检查**:
- 加载并解析配置（默认值 + 配置文件 + `GCOP__*` 环境变量覆盖）
- 列出从 provider 链中成功实例化的 providers
- 按配置的 provider 链（`default_provider` + `fallback_providers`）验证 provider 连通性
- 只要至少有一个成功实例化的 provider 验证成功就会返回成功

**示例输出**:
```
[1/2] Loading configuration...
✓ Configuration loaded successfully

Configured providers:
  • claude

[2/2] Testing provider connection...
✓ Provider 'claude' validated successfully
```

**何时使用**:
- 编辑配置后
- 排查连接问题
- 验证 API keys

## 参考

- [Provider 健康检查](../provider-health.md) - 验证流程与 endpoint 检查
- [配置指南](../configuration.md) - 完整配置说明
- [LLM Providers](../providers.md) - Provider 配置示例
