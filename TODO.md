# gcop-rs 功能建议 & Issue 想法

## 1. 🎯 高优先级功能建议

### 1.1 git hook 集成 (#feat) ✅ 已实现
**描述**: 支持作为 git prepare-commit-msg hook 运行，实现真正的 "commit 前自动生成"

**使用场景**:
```bash
# 安装 hook
gcop-rs hook install

# 之后 git commit 会自动触发 gcop 生成消息
```

**实现思路**:
- 新增 `gcop-rs hook install/uninstall` 命令
- 读取 `.git/hooks/prepare-commit-msg` 并注入 gcop 调用
- 支持 `--hook` 模式：从 STDIN/文件读取 commit message 并修改

---

### 1.2 Commit Message 模板/预设 (#feat) 🟡 部分实现
**描述**: 支持保存和快速切换不同的 commit message 风格预设

**使用场景**:
```bash
# 使用预设风格
gcop-rs commit --style angular    # Angular 规范
gcop-rs commit --style emoji      # Gitmoji 风格
gcop-rs commit --style minimal    # 极简风格
```

**配置示例**:
```toml
[commit.styles.angular]
template = """{{type}}({{scope}}): {{description}}

{{body}}

{{footer}}"""

[commit.styles.emoji]
template = """:{{emoji}}: {{description}}"""
```

---

### 1.3 批量 Commit/Interactive Rebase 支持 (#feat) ❌
**描述**: 支持交互式选择多个 staged chunks 分别生成 commit

**使用场景**:
```bash
# 交互式选择要 commit 的 hunks
gcop-rs commit --interactive
# 或
gcop-rs commit -i
```

**交互流程**:
1. 显示每个 changed file 的 hunks
2. 用户选择 y/n/s（是/否/分割）
3. 对选中的 hunks 分组生成多个 commit message
4. 逐个 commit

---

## 2. 🔧 中优先级改进

### 2.1 历史 Commit 修复/重写 (#feat) ❌
**描述**: 支持 AI 辅助修改历史 commit message

**使用场景**:
```bash
# 修复最近一个 commit 的消息
gcop-rs amend

# 修复指定 commit
gcop-rs rewrite HEAD~3

# 批量修复整个分支的 commit message（交互式）
gcop-rs rewrite main..feature --interactive
```

---

### 2.2 PR/MR 描述生成 (#feat) ❌
**描述**: 基于分支 commits 自动生成 Pull Request 描述

**使用场景**:
```bash
# 生成 PR 描述（markdown 格式）
gcop-rs pr-description
# 或
gcop-rs pr-desc --base main --head feature-branch

# 输出到剪贴板
gcop-rs pr-desc | pbcopy  # macOS
```

---

### 2.3 Commit 搜索/过滤 (#feat) ❌
**描述**: 自然语言搜索 commit history

**使用场景**:
```bash
# 搜索相关 commit
gcop-rs log "auth related changes"
gcop-rs log "fix memory leak"

# 使用 AI 语义搜索而非简单字符串匹配
```

---

### 2.4 代码变更摘要（Changelog 生成）(#feat) ❌
**描述**: 基于 commits 自动生成 CHANGELOG 或版本发布说明

**使用场景**:
```bash
# 生成版本变更日志
gcop-rs changelog --since v0.10.0 --to v0.11.0

# 生成未发布变更预览
gcop-rs changelog --unreleased
```

---

## 3. 🐛 潜在 Bug/改进点

### 3.1 并发安全 (#bug?) ✅ 已有方案
**位置**: `src/config/global.rs`

**问题**: 全局配置使用 `Arc<ArcSwap<AppConfig>>`，但在极端并发场景下可能存在时序问题。

**建议**: 检查配置热重载时的并发安全性，考虑使用 `RwLock` 或增加测试覆盖。

---

### 3.2 Diff 大小限制处理 (#enhancement) ✅ 已实现
**位置**: `src/commands/mod.rs` - `smart_truncate_diff`

**当前**: 自动生成文件降级为 summary，但 binary 文件可能被忽略

**建议**: 
- 检测 binary 文件并在 diff 中标记
- 对图片等 binary 文件可以提供 base64 或哈希摘要给 LLM

---

### 3.3 Provider 降级时丢失原始错误 (#bug?) ✅ 已实现
**位置**: `src/llm/provider/fallback.rs`

**问题**: fallback provider 切换时，原始错误信息可能被掩盖，用户不知道第一个 provider 为什么失败。

**建议**: 收集所有尝试的失败原因，最后汇总报告。

---

## 4. 💡 低优先级/脑洞功能

### 4.1 Commit streak 统计 (#feat) ✅ 已实现
**描述**: 类似 GitHub 的 contribution graph，但本地统计

```bash
gcop-rs streak
# 显示最近 30 天的 commit 热力图
```

---

### 4.2 Commit 质量评分 (#feat) ❌
**描述**: AI 对已生成的 commit message 打分

```bash
gcop-rs commit --quality-check
# 生成消息后询问 AI "这条 message 质量如何，如何改进"
```

---

### 4.3 多语言支持增强 (#i18n) 🟡 部分实现
**描述**: commit message 支持自动翻译成多种语言

```bash
gcop-rs commit --lang zh-cn  # 生成中文 commit message
gcop-rs commit --lang en     # 生成英文
```

---

### 4.4 Team Convention 检查 (#feat) ❌
**描述**: 检查 commit message 是否符合团队规范

```bash
# 配置团队规范
gcop-rs team init  # 创建 .gcop/team.toml

# 检查是否符合
gcop-rs team check HEAD~5..HEAD
```

---

## 5. 🔒 安全建议

### 5.1 API Key 掩码 (#security) ✅ 已实现
**位置**: 日志和错误处理

**建议**: 所有 API key 在日志中应自动掩码显示（如 `sk-ant-...xxxx`）

---

### 5.2 敏感文件检测 (#security) ❌
**描述**: 在生成 commit message 前检测 diff 中是否包含敏感信息

```toml
[security]
sensitive_patterns = ["password", "api_key", "secret"]
block_commit_on_detection = true  # 发现时阻止 commit
```

---

## 快速参考：优先级排序

| 优先级 | Issue | 影响 | 工作量 | 状态 |
|--------|-------|------|--------|------|
| ⭐⭐⭐ | git hook 集成 | 高 | 中 | ❌ |
| ⭐⭐⭐ | Commit 模板/预设 | 高 | 低 | 🟡 有 Convention 配置，缺 CLI --style 切换 |
| ⭐⭐ | PR 描述生成 | 中 | 中 | ❌ |
| ⭐⭐ | 历史 commit 重写 | 中 | 高 | ❌ |
| ⭐⭐ | Provider 错误汇总 | 中 | 低 | ✅ fallback.rs 已收集错误 |
| ⭐ | 代码变更摘要 | 低 | 中 | ❌ |
| ⭐ | API Key 掩码 | 安全 | 低 | ✅ mask_api_key + Debug impl |
| ⭐ | 敏感文件检测 | 安全 | 中 | ❌ |

---

*这些是基于代码结构的一些想法，可以根据实际需求和优先级选择实现。*