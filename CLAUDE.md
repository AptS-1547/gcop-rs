# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with this repository.

## Project Overview

`gcop-rs` 是一个用 Rust 编写的 AI 驱动 Git CLI，核心能力：

- 生成 commit message（`commit`）
- AI 代码审查（`review`）
- 配置管理（`init` / `config`）
- Git alias 管理（`alias`）
- 仓库统计（`stats`）
- Git hook 管理（`hook`）

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

# lint（CI 要求 -D warnings）
cargo clippy --all-targets --all-features -- -D warnings
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
│   ├── hook.rs                 # prepare-commit-msg hook 管理
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
├── workspace/
│   ├── mod.rs                  # WorkspaceInfo / PackageScope / detect_workspace()
│   ├── detector.rs             # 检测 Cargo/Pnpm/Npm/Lerna/Nx/Turbo workspace
│   ├── matcher.rs              # changed files → package 映射
│   └── scope.rs                # commit scope 推断（1 包→短名, 2-3→逗号, 4+→None）
├── llm/
│   ├── mod.rs                  # LLMProvider trait / StreamChunk / ProgressReporter / ScopeInfo
│   ├── message.rs              # LLM 消息结构
│   ├── prompt.rs               # prompt 组装（含 workspace scope 注入）
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

### 7) Monorepo workspace 支持

自动检测 6 种 monorepo 类型：Cargo、Pnpm、Npm、Lerna、Nx、Turbo。

**流程**：`detect_workspace()` → `map_files_to_packages()` → `infer_scope()` → 注入 LLM prompt。

- 检测逻辑在 `src/workspace/detector.rs`，按优先级扫描仓库根目录的配置文件
- Scope 推断规则：1 包→包短名，2-3 包→逗号分隔，4+ 包→None（让 LLM 自行判断）
- Scope 信息注入 prompt 的 user message（`## Workspace:` 段），不改 system prompt
- 所有检测错误非致命：`tracing::warn!` + 返回 None，不影响正常流程
- `-v` 模式下输出检测结果（`tracing::debug!`），默认静默
- 支持配置覆盖：`[workspace]` 段可手动指定 `members` 和 `scope_mappings`
- `CommitContext` 通过 `scope_info: Option<ScopeInfo>` 携带 workspace 信息
- 集成测试：`tests/workspace_e2e_test.rs`（14 个端到端测试覆盖所有类型）

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

### Project-level config

项目根目录下 `.gcop/config.toml` 可覆盖用户配置（如自定义 prompt、provider 等），优先级高于用户配置但低于环境变量。

---

## CI Checks

PR 合入 master 前需通过以下检查（`.github/workflows/ci.yml`）：

- **check**：`cargo fmt --check` + `cargo clippy --all-targets --all-features -- -D warnings`
- **test**：三平台矩阵（ubuntu / macOS / Windows），`cargo test --all-features`
- **audit**：`rustsec/audit-check` 安全扫描
- **coverage**：`cargo-llvm-cov` 生成覆盖率上传 Codecov
- **msrv**：Rust 1.92.0 兼容性检查

---

## Tests

- 单元测试：`#[cfg(test)] mod tests` 在各模块内
- 集成测试：`tests/` 目录，按功能组织（`commit_integration_test.rs`、`review_command_test.rs`、`workspace_e2e_test.rs` 等）
- `tests/test_git_ops.rs` 提供集成测试用的 Git 操作辅助
- `tests/workspace_e2e_test.rs` 覆盖 6 种 monorepo 类型的端到端检测 + scope 推断

运行集成测试时注意：需要 git 可用环境，部分测试会创建临时 git 仓库。

---

## i18n

翻译文件在 `locales/`（`en.yml`、`zh-CN.yml`）。使用 `rust-i18n` crate，代码中用 `t!("key")` 宏引用。添加新的用户可见字符串时需同步更新两个语言文件。

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
