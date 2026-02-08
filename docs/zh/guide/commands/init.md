# init

初始化 gcop-rs 配置。

**语法**:
```bash
gcop-rs init [OPTIONS]
```

**说明**:

默认执行用户级交互式初始化，指导你完成：
1. 创建配置目录（平台特定位置）
2. 复制示例配置文件
3. 设置安全文件权限（仅 Unix/Linux/macOS）
4. 可选安装 git 别名

使用 `--project` 可初始化仓库级配置（`.gcop/config.toml`），用于团队共享的非敏感设置。

**选项**:

| 选项 | 说明 |
|------|------|
| `--force`, `-f` | 强制覆盖已有配置文件 |
| `--project` | 在当前 Git 仓库初始化项目级 `.gcop/config.toml` |

> **注意**：如果当前目录不在 Git 仓库中，`--project` 会回退到当前目录，并创建 `./.gcop/config.toml`。

## 用户级示例（Linux）

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

## 项目级示例

```bash
$ gcop-rs init --project

✓ 已创建项目配置目录: /path/to/repo/.gcop
✓ 已创建项目配置文件: /path/to/repo/.gcop/config.toml

ℹ 下一步:
  1. 编辑 .gcop/config.toml，设置团队约定（prompt / convention / review 规则）
  2. API key 请放在用户级配置或环境变量中（不要写入项目配置）
```

**创建的内容**:
- `gcop-rs init`（默认）：平台特定位置的用户配置（来自 `examples/config.toml.example`）
- `gcop-rs init --project`：仓库级配置 `.gcop/config.toml`（来自 `examples/project-config.toml.example`）
- Git 别名写入 `~/.gitconfig`（仅默认交互模式，且你选择安装时）

**何时使用**: 首次设置或从头重新配置时。

## 参考

- [命令总览](../commands.md)
- [配置指南](../configuration.md)
