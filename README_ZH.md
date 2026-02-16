# gcop-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/gcop-rs)](https://crates.io/crates/gcop-rs)
[![Downloads](https://img.shields.io/crates/d/gcop-rs)](https://crates.io/crates/gcop-rs)
[![CI](https://github.com/AptS-1547/gcop-rs/workflows/CI/badge.svg)](https://github.com/AptS-1547/gcop-rs/actions)

AI 驱动的 Git 命令行工具 — 生成 commit message、审查代码、管理工作流，全在终端完成。使用 Rust 编写。

> 受 [Undertone0809](https://github.com/Undertone0809) 的 [gcop](https://github.com/Undertone0809/gcop) 启发的 Rust 重写版。

**[English](README.md)** | **[文档站点](https://gcop.docs.esap.cc/zh/)**

## 功能特性

- **AI 生成提交信息** — 通过 Claude、OpenAI、Gemini 或 Ollama 生成符合规范的 commit message
- **代码审查** — AI 驱动的代码审查，关注安全性与性能问题
- **Monorepo 支持** — 自动检测 Cargo、Pnpm、Npm、Lerna、Nx、Turbo 工作区并推断 commit scope
- **Git 别名** — `git c`、`git r`、`git acp` 等快捷方式简化工作流
- **Git Hook** — `prepare-commit-msg` hook，无缝集成编辑器提交流程
- **自定义 Provider** — 支持任意 OpenAI/Claude 兼容的 API（DeepSeek、自定义端点等）
- **自定义 Prompt** — 模板变量自定义 commit 和 review 的 prompt
- **项目级配置** — 项目根目录 `.gcop/config.toml` 覆盖用户配置
- **GPG 签名** — 通过原生 git 完整支持 GPG 提交签名
- **精美界面** — Spinner 动画、流式输出、彩色文本、交互式菜单

## 快速开始

### 1. 安装

```bash
# Homebrew (macOS/Linux)
brew tap AptS-1547/tap
brew install gcop-rs

# pipx (Python 用户)
pipx install gcop-rs

# cargo-binstall (预编译二进制，无需编译)
cargo binstall gcop-rs

# cargo install (从源码编译)
cargo install gcop-rs
```

更多安装方式见[安装指南](https://gcop.docs.esap.cc/zh/guide/installation)。

### 2. 配置

```bash
gcop-rs init
```

交互式向导会创建配置文件，并可选安装 git 别名。

也可以手动配置 — 使用 `gcop-rs config edit` 在系统编辑器中打开配置文件：

```toml
[llm]
default_provider = "claude"

[llm.providers.claude]
api_key = "sk-ant-your-key-here"
model = "claude-sonnet-4-5-20250929"
```

配置文件位置：`~/.config/gcop/`（Linux）、`~/Library/Application Support/gcop/`（macOS）、`%APPDATA%\gcop\config\`（Windows）。

环境变量覆盖：`GCOP__LLM__PROVIDERS__CLAUDE__API_KEY` 等。详见[配置指南](https://gcop.docs.esap.cc/zh/guide/configuration)。

### 3. 使用

```bash
git add .
gcop-rs commit            # AI 生成 commit message → 确认 → 提交
gcop-rs review changes    # AI 审查工作区变更

# 或使用别名（gcop-rs alias 安装后）：
git c                     # = gcop-rs commit
git acp                   # 添加所有 → AI 提交 → 推送
```

commit 流程是交互式的 — 生成后可以选择**接受**、**编辑**、**重试**或**带反馈重试**（如 "用中文"、"更简洁"）来逐步优化结果。

## 命令

| 命令 | 说明 |
|------|------|
| `gcop-rs commit` | 为暂存变更生成 AI commit message |
| `gcop-rs review <target>` | 审查 `changes` / `commit <hash>` / `range <a..b>` / `file <path>` |
| `gcop-rs init` | 交互式配置初始化 |
| `gcop-rs config edit` | 编辑配置（保存后自动校验） |
| `gcop-rs config validate` | 校验配置并测试 provider 连接 |
| `gcop-rs alias` | 安装 / 列出 / 删除 git 别名 |
| `gcop-rs stats` | 仓库提交统计 |
| `gcop-rs hook install` | 安装 `prepare-commit-msg` hook |
| `gcop-rs hook uninstall` | 卸载 hook |

全局参数：`-v` 详细输出、`--provider <name>` 覆盖 provider、`--format text|json|markdown` 输出格式、`--dry-run` 预览不提交。

详见[命令参考](https://gcop.docs.esap.cc/zh/guide/commands)。

## Git 别名

通过 `gcop-rs alias` 或 `gcop-rs init` 时安装。

| 别名 | 操作 |
|------|------|
| `git c` | AI 提交 |
| `git r` | AI 审查变更 |
| `git s` | 仓库统计 |
| `git ac` | 添加所有 + AI 提交 |
| `git cp` | AI 提交 + 推送 |
| `git acp` | 添加所有 + AI 提交 + 推送 |
| `git gconfig` | 编辑 gcop-rs 配置 |
| `git p` | 推送 |
| `git pf` | 强制推送（`--force-with-lease`） |
| `git undo` | 撤销最后一次提交（保留暂存） |

管理：`--list`、`--force`、`--remove --force`。详见[别名指南](https://gcop.docs.esap.cc/zh/guide/aliases)。

## 文档

- [安装指南](https://gcop.docs.esap.cc/zh/guide/installation) — 所有安装方式
- [配置参考](https://gcop.docs.esap.cc/zh/guide/configuration) — 完整配置说明
- [命令参考](https://gcop.docs.esap.cc/zh/guide/commands) — 详细命令文档
- [Provider 设置](https://gcop.docs.esap.cc/zh/guide/providers) — 配置 LLM 提供商（Claude、OpenAI、Gemini、Ollama、自定义）
- [自定义 Prompt](https://gcop.docs.esap.cc/zh/guide/prompts) — 模板变量和示例
- [Git 别名](https://gcop.docs.esap.cc/zh/guide/aliases) — 完整别名参考
- [故障排除](https://gcop.docs.esap.cc/zh/guide/troubleshooting) — 常见问题和解决方案

## 系统要求

- **Git** 2.0+
- **API Key**：至少一个 provider（Claude、OpenAI、Gemini），或本地 [Ollama](https://ollama.ai)
- **Rust** 1.88.0+（仅从源码编译时需要）

## 许可证

MIT — 详见 [LICENSE](LICENSE)。

## 贡献者

本项目是受 [Undertone0809](https://github.com/Undertone0809) 的 [gcop](https://github.com/Undertone0809/gcop) 启发的 Rust 重写版。使用 AI 生成 commit message 的核心理念源自该项目。

**作者**：[AptS-1547](https://github.com/AptS-1547)、[AptS-1738](https://github.com/AptS-1738)、[uaih3k9x](https://github.com/uaih3k9x)
