# TODO

## Issue #16: [refactor] 可维护性增强

- Issue: https://github.com/AptS-1547/gcop-rs/issues/16
- 现状: Issue body 为空（没有明确痛点/范围/验收标准）。下面的 TODO 基于对当前仓库的实际调查结果整理，优先列“低风险高收益”的重构点。

### 调查结果（基于当前仓库）

#### 质量基线

- `cargo test`：通过（单测 + 集成测试 + doctest 全部通过）。
- `cargo fmt --check`：通过。
- `cargo clippy --all-targets --all-features`：仅 1 个 warning：
  - `clippy::should_implement_trait`：`src/commands/format.rs` 里的 `OutputFormat::from_str` 命名容易和 `FromStr::from_str` 混淆，建议实现 `std::str::FromStr` 或改名为 `parse`/`from_cli_str`。

#### 维护热点（按行数粗略排序）

- `src/llm/provider/base.rs`（~679 行）：HTTP 请求/重试/Retry-After/解析/错误处理集中，且包含 `spinner` 依赖（UI 耦合）。
- `src/commands/commit.rs`（~508 行）：交互流程 + message 生成（流式/非流式）+ JSON 输出都在一个文件内。
- `src/llm/provider/openai.rs` / `src/llm/provider/claude.rs`（~4xx 行）：非流式走统一 `send_llm_request`，流式路径单独实现；目前流式错误传播不一致（见下）。
- `src/main.rs`（~205 行）：大量重复的错误打印/建议提示/exit code 分支。

#### 已确认的“重复/耦合/不一致”点（适合纳入 #16）

1) CLI 路由与错误处理重复
- `src/main.rs` 内部对每个子命令都重复：调用命令 → match 错误 → 打印 tip → `std::process::exit`。
- `std::process::exit` 在 `main.rs` 出现多处（维护时容易漏改某个分支的行为）。

2) JSON wrapper 重复（但已有可复用的 error code 映射）
- `commit/review/stats` 都有 `{ success, data, error }` wrapper（`CommitJsonOutput` / `ReviewJsonOutput` / `StatsJsonOutput`）。
- 已存在 `src/commands/json.rs` 的 `ErrorJson` + `error_to_code()`，说明“统一 JSON 输出”已经有一半基础设施，适合把 wrapper 再统一掉。

3) 默认值来源重复（潜在漂移风险）
- 默认值在 `src/config/schema.rs` 的 `Default` 实现里有一份；
- 同时 `src/config/mod.rs:load_config()` 又用 `config::Config::builder().set_default(...)` 维护了一份；
- 目前靠测试与人工同步，长期维护建议收敛为单一来源或加入“默认值一致性”测试。

4) 流式输出错误传播不一致（可能导致“静默失败/截断消息”）
- `src/ui/streaming.rs` 设计上支持 `StreamChunk::Error` 并会返回 `Err`；
- 但 `src/llm/provider/streaming.rs` 只发送 `Delta/Done`，不会发送 `Error`；
- `src/llm/provider/openai.rs` 的流式处理在后台任务里遇到错误只打印（`ui::colors::error`），不会向 channel 发送 `StreamChunk::Error`，调用方可能把“半截输出”当成成功结果。
- 这属于“结构设计不一致”，同时也是可靠性问题，建议优先修正（P0）。

5) 测试有效性小问题
- `src/commands/commit_state_machine.rs` 的 `test_generating_max_retries_exceeded` 只断言 `is_err()`，但对错误变体的断言分支是 `if let Err(GcopError::Other(..))`，这会导致测试“通过但不验证关键行为”。建议改成强断言 `GcopError::MaxRetriesExceeded(_)`。

6) 仓库文档引用一致性
- `docs/release-notes/v0.6.0.md` 和 `docs/zh/release-notes/v0.6.0.md` 引用了 `TODO.md`，但仓库里原本没有（现在补上了）。建议后续把 TODO 作为长期 backlog 维护入口，避免文档失真。

### 建议补充到 Issue #16 的“定义”（让重构有边界）

- 目标: 对外行为不变（CLI 参数、配置兼容、JSON schema、默认输出格式），只做结构整理 + 一致性修复 + 回归测试增强。
- 验收标准（建议写进 issue）：
  - `cargo test`/`cargo fmt --check`/`cargo clippy --all-targets --all-features` 全部通过且 clippy 0 warning
  - JSON 输出 wrapper 统一（字段名/错误码一致）
  - 流式模式在网络错误时能明确失败（返回非 0 或输出 JSON error），不会产生“静默截断”

### Backlog（按优先级拆分成小 PR 更好 review）

#### P0（优先做：收益高 + 风险可控）

- [ ] 修复“流式错误不向上游传播”的一致性问题（`openai`/`claude`/fallback 行为对齐）
  - 方向 A：在 provider 的后台任务里捕获错误后发送 `StreamChunk::Error(...)` 再关闭 channel
  - 方向 B：让 `process_openai_stream/process_claude_stream` 在内部遇到错误时发送 `Error`（需要约定错误语义）
  - 补回归：模拟网络中断/解析错误，确保 `StreamingOutput::process()` 返回 `Err`
- [ ] 处理 clippy 警告：为 `OutputFormat` 实现 `std::str::FromStr` 或改名避免歧义（`src/commands/format.rs`）
- [ ] 强化 `commit_state_machine` 的最大重试测试断言（`src/commands/commit_state_machine.rs`）

#### P1（纯重构：减少重复，提高可读性）

- [ ] 统一 JSON 输出 wrapper（复用 `src/commands/json.rs`）
  - 引入通用 `JsonOutput<T>`（或类似）+ 通用 `print_json_success/print_json_error`
  - `commit/review/stats` 只保留各自的 `data` 结构体（或直接复用现有类型）
- [ ] 抽离 `main.rs` 的“命令路由 + 错误输出 + exit code”重复逻辑
  - 目标：`main.rs` 只做日志初始化 + config 加载 + `app::run(cli)`，退出码与输出规则集中维护

#### P2（结构优化：让文件变薄、职责更清晰）

- [ ] 拆分 `src/commands/commit.rs`（按职责拆模块，不改行为）
  - message 生成（流式/非流式）
  - UI 交互（菜单/编辑器）
  - JSON 输出
  - git commit 执行
- [ ] 消除 commit message 生成路径里的重复（`generate_message` vs `generate_message_no_streaming`）
  - 提取 “构建 CommitContext + verbose prompt 打印” 成共享函数

#### P3（长期：降低层间耦合，提升可测试性）

- [ ] 降低 provider 层对 UI 的依赖
  - 当前 `LLMProvider`/`send_llm_request`/`streaming.rs` 都透传 `colored` 或 `Spinner`，建议改为更抽象的回调/事件（UI 在 commands 层消费）
- [ ] 配置默认值收敛为单一来源（避免 `schema.rs` 与 `load_config()` 漂移）
  - 方案：`load_config()` 基于 `AppConfig::default()` 合并文件/环境变量，或者增加“默认值一致性”单测
