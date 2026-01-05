# gcop-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/gcop-rs)](https://crates.io/crates/gcop-rs)
[![Downloads](https://img.shields.io/crates/d/gcop-rs)](https://crates.io/crates/gcop-rs)
[![CI](https://github.com/AptS-1547/gcop-rs/workflows/CI/badge.svg)](https://github.com/AptS-1547/gcop-rs/actions)

AI 驱动的 Git 提交信息生成器和代码审查工具，使用 Rust 编写。

> **说明**: 这是对原 [gcop](https://github.com/Undertone0809/gcop) 项目（Python 版本）的 Rust 重写。由于原项目不再积极维护，本项目旨在提供更好的性能、可靠性和可维护性。

**[English](README.md)** | **[文档](docs/zh/guide/)**

## 功能特性

- 🤖 **AI 生成提交信息** - 使用 Claude、OpenAI 或 Ollama 生成符合规范的提交信息
- 🔍 **代码审查** - AI 驱动的代码审查，关注安全性和性能问题
- 🎯 **Git 别名** - 便捷的快捷方式，如 `git c`、`git r`、`git acp` 简化工作流程
- 🚀 **快速设置** - 交互式 `init` 命令快速配置
- 🔧 **自定义 Provider** - 支持任意 OpenAI/Claude 兼容的 API（DeepSeek、自定义端点等）
- 📝 **自定义 Prompt** - 使用模板变量自定义生成和审查的 prompt
- ⚙️  **灵活配置** - 通过配置文件或环境变量配置
- 🎨 **精美界面** - Spinner 动画、彩色输出、交互式提示
- 🐛 **调试模式** - 详细日志，可查看完整的请求/响应
- 🔐 **GPG 签名** - 完整支持 GPG 提交签名（通过原生 git 命令）

## 快速开始

### 1. 安装

```bash
# Homebrew (macOS/Linux)
brew tap AptS-1547/gcop-rs
brew install gcop-rs

# pipx (Python 用户)
pipx install gcop-rs

# cargo-binstall (无需编译)
cargo binstall gcop-rs

# cargo install (从源码编译)
cargo install gcop-rs
```

其他安装方式详见 [docs/zh/guide/installation.md](docs/zh/guide/gui deinstallation.md)。

### 2. 配置

**方式 1: 快速设置（推荐）**

```bash
gcop-rs init
```

交互式向导将：
- 在平台特定位置创建配置目录和文件
- 设置安全文件权限（Unix/Linux/macOS）
- 可选安装便捷的 git 别名

**方式 2: 手动设置**

使用 `gcop-rs config edit` 在系统编辑器中打开配置文件，或手动创建于：
- **Linux**: `~/.config/gcop/config.toml`
- **macOS**: `~/Library/Application Support/gcop/config.toml`
- **Windows**: `%APPDATA%\gcop\config.toml`

```toml
[llm]
default_provider = "claude"

[llm.providers.claude]
api_key = "sk-ant-your-key-here"
model = "claude-sonnet-4-5-20250929"
```

或使用环境变量：
```bash
export ANTHROPIC_API_KEY="sk-ant-your-key"
```

详见 [docs/zh/guide/configuration.md](docs/zh/guide/configuration.md)。

### 3. 使用

```bash
# 生成提交信息
git add .
gcop-rs commit
# 或使用别名: git c

# 审查未提交的变更
gcop-rs review
# 或使用别名: git r

# 完整工作流
git acp  # 添加所有、AI 提交、推送

# 使用不同的 provider
gcop-rs --provider openai commit
```

## Git 别名

gcop-rs 提供便捷的 git 别名来简化工作流程。

### 安装

```bash
# 安装所有别名
gcop-rs alias

# 或在初始化时安装
gcop-rs init  # 会提示是否安装别名
```

### 使用

安装后，你可以使用这些快捷方式：

```bash
git c          # AI 生成提交信息并提交
git r          # AI 审查未提交的变更
git s          # 显示仓库统计
git ac         # 添加所有变更并用 AI 提交
git cp         # AI 提交并推送
git acp        # 添加、AI 提交并推送
git gconfig    # 编辑 gcop-rs 配置
git p          # 推送到远程
git pf         # 强制推送（使用 --force-with-lease 更安全）
git undo       # 撤销最后一次提交（保留暂存的变更）
```

### 管理

```bash
# 列出所有可用的别名
gcop-rs alias --list

# 重新安装（覆盖冲突）
gcop-rs alias --force

# 删除所有 gcop-rs 别名
gcop-rs alias --remove --force
```

详细信息见 [docs/zh/guide/aliases.md](docs/zh/guide/aliases.md)。

## 命令说明

### `gcop-rs init`

初始化 gcop-rs 配置。

```bash
gcop-rs init
```

交互式设置向导：
- 创建配置目录
- 复制示例配置
- 设置安全文件权限
- 可选安装 git 别名

---

### `gcop-rs commit`

为暂存的变更生成 AI 驱动的提交信息。

```bash
gcop-rs commit              # 生成、审查并提交
gcop-rs commit --no-edit    # 跳过编辑器
gcop-rs commit --yes        # 跳过确认
gcop-rs commit --dry-run    # 仅输出信息，不提交
gcop-rs -v commit           # 详细模式
```

**交互式工作流**:

生成提交信息后，你可以选择：
- **Accept（接受）** - 使用生成的信息
- **Edit（编辑）** - 打开编辑器手动修改（编辑后返回菜单）
- **Retry（重试）** - 不带反馈重新生成
- **Retry with feedback（带反馈重试）** - 提供自定义指令（如 "用中文"、"更简洁"、"更详细"）。反馈会累积，多次重试可逐步优化结果
- **Quit（退出）** - 取消提交

示例：
```bash
$ git add .
$ gcop-rs commit

ℹ 生成的提交信息:
feat(auth): 实现 JWT 令牌验证

选择下一步操作:
> 接受
  编辑
  重试
  带反馈重试
  退出
```

---

### `gcop-rs review`

使用 AI 审查代码变更。

```bash
gcop-rs review                   # 审查未提交的变更
gcop-rs review --commit <hash>   # 审查特定 commit
gcop-rs review --range main..dev # 审查 commit 范围
gcop-rs review --file src/main.rs # 审查特定文件
```

**输出格式**: `--format text|json|markdown`

---

### `gcop-rs config`

管理配置。

```bash
# 在默认编辑器中编辑配置文件（带校验）
gcop-rs config edit

# 验证配置并测试 provider 连接
gcop-rs config validate

# 显示当前配置
gcop-rs config show
```

`config edit` 会在保存后校验配置（类似 `visudo`），即使配置损坏也能运行。

> **提示**: 建议始终使用 `gcop-rs config edit` 而不是直接编辑配置文件，以避免语法错误。

---

### `gcop-rs alias`

管理 git 别名。

```bash
gcop-rs alias                       # 安装所有别名
gcop-rs alias --list                # 列出可用的别名
gcop-rs alias --force               # 覆盖冲突
gcop-rs alias --remove --force      # 删除所有别名
```

提供便捷的快捷方式，如 `git c`、`git r`、`git acp` 等。

详见 [docs/zh/guide/aliases.md](docs/zh/guide/aliases.md)。

---

### `gcop-rs stats`

显示仓库提交统计。

```bash
gcop-rs stats                       # 显示统计（文本格式）
gcop-rs stats --format json         # 输出为 JSON
gcop-rs stats --format markdown     # 输出为 Markdown
gcop-rs stats --author "name"       # 按作者过滤
```

显示内容：
- 总提交数和贡献者数
- 仓库时间跨度
- 贡献者排行榜（按提交数）
- 最近活动（近 4 周）ASCII 柱状图

## 配置

配置文件位置（平台特定）：
- **Linux**: `~/.config/gcop/config.toml`
- **macOS**: `~/Library/Application Support/gcop/config.toml`
- **Windows**: `%APPDATA%\gcop\config.toml`

使用 Claude API 的示例配置：

```toml
[llm]
default_provider = "claude"

[llm.providers.claude]
api_key = "sk-ant-your-key"
model = "claude-sonnet-4-5-20250929"
temperature = 0.3

[commit]
show_diff_preview = true
allow_edit = true

[review]
min_severity = "info"

[ui]
colored = true
```

完整配置参考见 [docs/zh/guide/configuration.md](docs/zh/guide/configuration.md)。

## 高级功能

### 自定义 Provider

添加任意 OpenAI 或 Claude 兼容的 API：

```toml
[llm.providers.deepseek]
api_style = "openai"
api_key = "sk-your-deepseek-key"
endpoint = "https://api.deepseek.com/v1/chat/completions"
model = "deepseek-chat"
```

更多示例见 [docs/zh/guide/providers.md](docs/zh/guide/providers.md)。

### 自定义 Prompt

自定义提交信息或审查的 prompt：

```toml
[commit]
custom_prompt = """
为以下变更生成中文提交信息：
{diff}

文件: {files_changed}
统计: +{insertions} -{deletions}
"""
```

模板变量和示例见 [docs/zh/guide/prompts.md](docs/zh/guide/prompts.md)。

### 调试模式

使用 `--verbose` 查看详细日志：

```bash
gcop-rs -v commit  # 显示 API 请求、响应和 prompts
```

> **安全提示**: verbose 模式（`-v` 或 `RUST_LOG=debug`）会在日志中打印完整的 API 请求和响应，可能包含：
> - 你的代码 diff 和变更内容
> - 错误信息中的部分 API key
> - 生成的 commit message
>
> 不要公开分享这些日志或将其提交到版本控制中。

## 文档

- **[安装指南](docs/zh/guide/installation.md)** - 详细的安装说明
- **[Git 别名指南](docs/zh/guide/aliases.md)** - Git 别名完整指南
- **[命令参考](docs/zh/guide/commands.md)** - 详细的命令文档
- **[配置参考](docs/zh/guide/configuration.md)** - 完整的配置指南
- **[Provider 设置](docs/zh/guide/providers.md)** - 配置 LLM 提供商
- **[自定义 Prompt](docs/zh/guide/prompts.md)** - 自定义 AI prompts
- **[故障排除](docs/zh/guide/troubleshooting.md)** - 常见问题和解决方案

## 系统要求

- Rust 1.70 或更高版本
- Git
- 至少一个 provider 的 API key（Claude、OpenAI 或本地 Ollama）

## 许可证

MIT License - 详见 LICENSE 文件。

## 作者

AptS:1547 (Yuhan Bian / 卞雨涵) <apts-1547@esaps.net>

---

**提示**: 运行 `gcop-rs --help` 查看所有命令，或在安装别名后使用 `git c` 快速提交！
