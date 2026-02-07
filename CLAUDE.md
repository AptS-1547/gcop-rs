# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

gcop-rs 是一个用 Rust 编写的 AI 驱动的 Git 工具，用于生成 commit message 和代码审查。这是原 Python 项目 [gcop](https://github.com/Undertone0809/gcop) 的重写版本。

## 常用命令

### 构建和运行
```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# 运行（开发模式）
cargo run -- --help
cargo run -- commit
cargo run -- review changes

# 安装本地版本
cargo install --path .
```

### 测试
```bash
# 运行所有测试（单元测试 + 集成测试）
cargo test

# 运行单个测试
cargo test test_name

# 运行特定模块的测试
cargo test config::tests::
cargo test git::

# 显示测试输出
cargo test -- --nocapture

# 运行集成测试（需要 git 环境）
cargo test --test integration_test
cargo test --test commit_integration_test
```

### Linting 和格式化
```bash
# 格式化代码
cargo fmt

# 检查格式
cargo fmt -- --check

# 运行 clippy（lint）
cargo clippy

# 检查代码（快速编译检查）
cargo check
```

### 文档开发
```bash
# 进入文档目录
cd docs

# 安装依赖
pnpm install

# 启动开发服务器
pnpm dev

# 构建文档
pnpm build

# 预览构建结果
pnpm preview
```

## 代码架构

### 核心模块

```
src/
├── main.rs                    # 入口点，CLI 路由和错误处理
├── lib.rs                     # 公开模块导出
├── cli.rs                     # Clap CLI 定义
├── commands/                  # 命令实现
│   ├── commit.rs              # commit 命令主逻辑
│   ├── commit_state_machine.rs # commit 流程状态机（纯函数式）
│   ├── review.rs              # review 命令
│   ├── config.rs              # config 命令
│   ├── init.rs                # init 命令
│   ├── alias.rs               # alias 命令
│   └── stats.rs               # stats 命令（仓库统计）
├── config/                    # 配置管理
│   ├── mod.rs                 # 配置加载（文件 + 环境变量）
│   └── schema.rs              # 配置数据结构
├── git/                       # Git 操作封装
│   ├── mod.rs                 # GitOperations trait 定义
│   ├── repository.rs          # git2 实现
│   ├── diff.rs                # diff 解析
│   └── commit.rs              # commit 执行
├── llm/                       # LLM 集成
│   ├── mod.rs                 # LLMProvider trait + StreamChunk 定义
│   ├── provider/
│   │   ├── base.rs            # 基础 provider 实现
│   │   ├── claude.rs          # Claude API
│   │   ├── openai.rs          # OpenAI API（支持流式输出）
│   │   ├── ollama.rs          # Ollama API
│   │   ├── streaming.rs       # SSE 流式响应解析
│   │   └── utils.rs           # 通用工具函数
│   ├── prompt.rs              # Prompt 模板
│   └── message.rs             # 消息格式
├── ui/                        # 用户界面
│   ├── spinner.rs             # Loading 动画
│   ├── prompt.rs              # 交互式 prompt
│   ├── editor.rs              # 编辑器集成
│   ├── streaming.rs           # 流式文本输出（打字效果）
│   └── colors.rs              # 颜色输出
└── error.rs                   # 统一错误类型
```

### 关键设计

#### 1. Commit 状态机模式

`commands/commit_state_machine.rs` 实现了一个纯函数式状态机来管理 commit 流程：

```rust
pub enum CommitState {
    Generating { attempt: usize, feedbacks: Vec<String> },
    WaitingForAction { message: String, ... },
    Accepted { message: String },
    Cancelled,
}

pub enum UserAction {
    Accept,
    Edit { new_message: String },
    Retry,
    RetryWithFeedback { feedback: Option<String> },
    Quit,
}
```

状态转换是纯函数，便于测试和推理。IO 操作由 `commands/commit.rs` 处理。

#### 2. Trait 抽象层

- **GitOperations** (`git/mod.rs`): 抽象 git 操作，支持 mock 测试
- **LLMProvider** (`llm/mod.rs`): 抽象 LLM API，支持多 provider

使用 `mockall` crate 在测试中 mock 这些 trait。

#### 3. 配置优先级

配置加载顺序（优先级从高到低）：
1. 环境变量 (`GCOP_*` 前缀，如 `ANTHROPIC_API_KEY`)
2. 配置文件（路径见下方"配置"章节）
3. 代码默认值

这允许灵活的配置管理，并支持 CI/CD 环境。

#### 4. 错误处理

统一使用 `error::GcopError` 和 `Result<T>` 类型。错误类型实现了 `suggestion()` 方法提供用户友好的解决建议。

#### 5. 流式输出

OpenAI provider 支持流式输出（SSE），相关代码：
- `llm/provider/streaming.rs`: SSE 流式响应解析
- `llm/mod.rs`: `StreamChunk` enum 定义流式数据块
- `ui/streaming.rs`: 流式文本输出（打字机效果）

#### 6. 测试策略

- 单元测试：直接在模块文件中 (`#[cfg(test)] mod tests`)
- 集成测试：`tests/` 目录
- Mock 支持：`mockall` feature (`feature = "test-utils"`)
- 环境变量测试：使用 `serial_test` 避免并发冲突

## 配置

配置文件位置（平台特定）：
- Linux: `~/.config/gcop/config.toml`
- macOS: `~/Library/Application Support/gcop/config.toml`
- Windows: `%APPDATA%\gcop\config.toml`

示例配置：
```toml
[llm]
default_provider = "claude"

[llm.providers.claude]
api_key = "sk-ant-your-key"
model = "claude-sonnet-4-5-20250929"

[commit]
max_retries = 10
show_diff_preview = true
```

## 开发注意事项

### 添加新的 LLM Provider

1. 在 `src/llm/provider/` 创建新文件
2. 实现 `LLMProvider` trait
3. 在 `src/llm/provider/mod.rs` 中注册
4. 在 `src/config/schema.rs` 添加配置结构

### 修改 Commit 流程

Commit 流程逻辑集中在两个地方：
- **状态转换逻辑**: `commands/commit_state_machine.rs` (纯函数)
- **IO 和渲染**: `commands/commit.rs` (async/await)

修改流程时，尽量保持状态机纯函数性质。

### 测试 Git 操作

集成测试会创建临时 git 仓库（使用 `tempfile` crate）。测试中的 git 操作是真实的，需要确保系统已安装 git。

### 发布流程

1. 更新 `Cargo.toml` 版本号
2. 更新 `python/pyproject.toml` 版本号（保持一致）
3. 更新 `CHANGELOG.md`
4. 创建 release notes (`docs/release-notes/` 和 `docs/zh/release-notes/`)
5. 提交并打 tag: `git tag v0.x.x`
6. 推送 tag 触发 CI: `git push origin v0.x.x`
   - CI 自动构建多平台二进制
   - CI 自动发布到 crates.io
   - CI 自动构建并发布 PyPI wheels（使用 maturin）

## 依赖关系

主要依赖：
- `clap`: CLI 框架
- `git2`: Git 操作（libgit2 bindings）
- `tokio`: 异步运行时
- `reqwest`: HTTP 客户端
- `serde`/`toml`: 配置序列化
- `dialoguer`: 交互式 prompt
- `indicatif`: 进度条和 spinner
- `mockall`: Mock 测试 (dev)

## 调试

> **安全提示**: verbose 模式会在日志中打印完整的 prompt 和 LLM 响应，可能包含代码片段。不要在公开场合分享这些日志。

启用详细日志：
```bash
gcop-rs -v commit           # 命令行参数
RUST_LOG=debug gcop-rs commit # 环境变量
```

日志使用 `tracing` crate，会显示 API 请求/响应和详细流程。
