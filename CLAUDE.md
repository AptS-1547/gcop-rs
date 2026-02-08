# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with this repository.

## Project Overview

`gcop-rs` 是一个用 Rust 编写的 AI 驱动 Git CLI，核心能力：

- 生成 commit message（`commit`）
- AI 代码审查（`review`）
- 配置管理（`init` / `config`）
- Git alias 管理（`alias`）
- 仓库统计（`stats`）

支持的 Provider：`claude` / `openai` / `ollama` / `gemini`。

Rust edition 2024，MSRV 1.93.0。i18n 通过 `rust-i18n` 实现，翻译文件在 `locales/`（en、zh-CN）。

---

## Common Commands

### Build & Run

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# 运行示例
cargo run -- --help
cargo run -- commit --help
cargo run -- review --help
cargo run -- review changes

# 本地安装
cargo install --path .
```

### Test

```bash
# 全量测试（单元 + 集成 + doctest）
cargo test

# 单个测试
cargo test test_name

# 某个模块
cargo test config::tests::
cargo test git::repository::tests::

# 查看测试输出
cargo test -- --nocapture
```

### Format & Lint

```bash
# 格式化
cargo fmt

# 检查格式
cargo fmt --all -- --check

# 编译检查
cargo check

# lint
cargo clippy
```

### Docs (VitePress)

```bash
cd docs
pnpm install
pnpm dev
pnpm build
pnpm preview
```

---

## Architecture (Quick Map)

```text
src/
├── main.rs                     # CLI 入口：加载配置、解析参数、命令路由
├── cli.rs                      # Clap 命令定义
├── lib.rs                      # library crate root
├── error.rs                    # GcopError 定义（thiserror）+ Result 别名
├── commands/
│   ├── commit.rs               # commit 主流程（IO + 渲染 + provider 调用）
│   ├── commit_state_machine.rs # commit 状态机（纯函数）
│   ├── review.rs               # review 主流程
│   ├── config.rs               # config edit/validate
│   ├── init.rs                 # 初始化配置
│   ├── alias.rs                # git alias 管理
│   ├── stats.rs                # 仓库统计
│   ├── options.rs              # 命令选项结构体
│   ├── format.rs               # 输出格式（text/json/markdown）
│   └── json.rs                 # JSON 输出辅助
├── config/
│   ├── structs.rs              # 配置结构定义
│   ├── loader.rs               # 配置加载（文件 + GCOP__ + CI 覆盖）
│   ├── global.rs               # 全局配置单例（可选接口）
│   └── tests.rs                # 配置模块测试
├── git/
│   ├── mod.rs                  # GitOperations trait
│   ├── repository.rs           # git2 实现
│   ├── diff.rs                 # diff 统计解析
│   └── commit.rs               # 执行 commit
├── llm/
│   ├── mod.rs                  # LLMProvider trait / StreamChunk / ProgressReporter
│   ├── message.rs              # LLM 消息结构
│   ├── prompt.rs               # prompt 组装
│   └── provider/
│       ├── claude.rs
│       ├── openai.rs
│       ├── ollama.rs
│       ├── gemini.rs
│       ├── fallback.rs         # provider 链式降级
│       ├── streaming.rs        # SSE 解析
│       ├── utils.rs            # provider 通用工具
│       ├── test_utils.rs       # provider 测试辅助（需 test-utils feature）
│       └── base/
│           ├── config.rs       # 请求配置
│           ├── response.rs     # 响应解析
│           ├── retry.rs        # 重试逻辑
│           └── validation.rs   # 响应校验
└── ui/
    ├── colors.rs               # 颜色方案
    ├── editor.rs               # 编辑器集成
    ├── prompt.rs               # 交互菜单
    ├── spinner.rs              # 加载动画
    └── streaming.rs            # 流式输出渲染
```

---

## Key Design Notes

### 1) Commit state machine

`commands/commit_state_machine.rs` 只负责状态转换（纯函数），
`commands/commit.rs` 负责 IO、LLM 调用、UI 渲染。

### 2) Trait-based boundaries

- `GitOperations`：隔离 git 实现，便于 mock
- `LLMProvider`：隔离不同 Provider（含 fallback）
- `ProgressReporter`：LLM 层向 UI 报告状态（重试、fallback 切换），解耦 LLM 与 UI

### 3) Output format policy

`OutputFormat` 统一管理 `text/json/markdown`：

- `json` / `markdown` 视为 machine-readable，默认禁用颜色和交互 UI 元素
- `commit --json` 走非交互流程，不会实际提交

### 4) Streaming behavior

- Streaming 支持：OpenAI / Claude / Gemini
- Ollama 当前不支持 streaming，会回退 spinner 模式

### 5) Error handling

统一使用 `GcopError`（`src/error.rs`），并提供本地化 message + suggestion。

### 6) Test utilities

`mockall` 隐藏在 `test-utils` feature flag 后面。dev-dependencies 中已通过 `gcop-rs = { path = ".", features = ["test-utils"] }` 启用。编写需要 mock trait 的测试时，相关 mock 类型只在该 feature 下可用。

---

## Configuration

### Config file locations

- Linux: `~/.config/gcop/config.toml`
- macOS: `~/Library/Application Support/gcop/config.toml`
- Windows: `%APPDATA%\gcop\config\config.toml`

### Load priority (high → low)

1. CI mode overrides (`CI=1` + `GCOP_CI_*`)
2. Environment overrides (`GCOP__*`, nested by `__`)
3. Config file (`config.toml`)
4. Defaults (serde/default impl)

### CI mode notes

- `GCOP_CI_PROVIDER` 支持：`claude` / `openai` / `ollama` / `gemini`
- 会注入 provider 名为 `ci`，并成为 `default_provider`

---

## Development Guidelines

### Adding a new Provider

1. 在 `src/llm/provider/` 新增实现文件
2. 在 `src/llm/provider/mod.rs` 注册模块与构建分支
3. 如需新增 API 风格，更新 `config::ApiStyle`
4. 补充 provider 测试与文档（`docs/guide/*` + `docs/zh/guide/*`）

### Changing commit flow

- 优先改状态机（纯转换逻辑）
- 再改 `commit.rs`（交互与副作用）
- 保持 `--json` / `--dry-run` 分支行为稳定

### Docs consistency

若改动了命令/配置/provider 行为，请同步更新：

- `docs/guide/*`
- `docs/zh/guide/*`
- `examples/config.toml.example`（及 `.zh`）

---

## Debugging & Safety

Verbose 模式会打印较多调试信息（含 prompt / API 响应片段），请避免公开分享敏感日志。

```bash
gcop-rs -v commit
gcop-rs -v review changes
RUST_LOG=debug gcop-rs commit
```

---

## Release Checklist (Quick)

1. 更新版本（`Cargo.toml` + `python/pyproject.toml`）
2. 更新 `CHANGELOG.md`
3. 更新 release notes（中英文）
4. 跑 `cargo test` / `cargo fmt --check`
5. 打 tag 并推送
