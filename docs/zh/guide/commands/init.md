# init

使用交互式向导初始化 gcop-rs 配置。

**语法**:
```bash
gcop-rs init [OPTIONS]
```

**说明**:

交互式设置，指导你完成：
1. 创建配置目录（平台特定位置）
2. 复制示例配置文件
3. 设置安全文件权限（仅 Unix/Linux/macOS）
4. 可选安装 git 别名

**选项**:

| 选项 | 说明 |
|------|------|
| `--force`, `-f` | 强制覆盖已有配置文件 |

**示例** (Linux):
```bash
$ gcop-rs init

✓ 已创建配置目录: /home/user/.config/gcop
✓ 已创建配置文件: /home/user/.config/gcop/config.toml
✓ 已设置文件权限: 600

ℹ 下一步:
  1. 编辑配置文件: gcop-rs config edit
  2. 为你首选的 provider 设置 API key
  3. 测试: gcop-rs commit --help

安装 git 别名？ (Y/n): y

[1/2] 正在安装 git 别名...
  ✓  git c          → AI 提交
  ✓  git r          → AI 审查
  ...

✓ 已安装 14 个别名
```

**创建的内容**:
- 配置文件位于平台特定位置（来自 `examples/config.toml.example`）
- Git 别名配置到 `~/.gitconfig`（如果选择安装）

**何时使用**: 首次设置或从头重新配置时。

## 参考

- [命令总览](../commands.md)
- [配置指南](../configuration.md)
