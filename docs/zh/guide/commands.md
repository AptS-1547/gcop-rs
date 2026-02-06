# 命令参考

所有 gcop-rs 命令和选项的完整参考。

## 全局选项

这些选项可以用于任何命令：

| 选项 | 说明 |
|------|------|
| `--provider <NAME>`, `-p` | 覆盖默认 LLM provider (claude, openai, ollama 或自定义) |
| `--verbose`, `-v` | 启用详细日志（显示 API 请求和响应） |
| `--help`, `-h` | 显示帮助信息 |
| `--version`, `-V` | 显示版本信息 |

**示例**:
```bash
gcop-rs --provider openai commit
gcop-rs -v review changes
```

---

## 命令

### init

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

---

### commit

生成 AI 驱动的提交信息并创建提交。

**语法**:
```bash
gcop-rs commit [OPTIONS] [FEEDBACK...]
```

**说明**:

分析暂存的变更，使用 AI 生成符合规范的提交信息，并在你批准后创建 git 提交。

**选项**:

| 选项 | 说明 |
|------|------|
| `--format <FORMAT>`, `-f` | 输出格式: `text`（默认）或 `json`（json 模式不会创建提交） |
| `--json` | `--format json` 的快捷方式 |
| `--no-edit`, `-n` | 跳过打开编辑器手动编辑 |
| `--yes`, `-y` | 跳过确认菜单并接受生成的信息 |
| `--dry-run`, `-d` | 仅生成并输出提交信息，不实际提交 |
| `--provider <NAME>`, `-p` | 使用特定的 provider（覆盖默认值） |

**反馈（可选）**:

你可以在选项后面追加一段自由文本，作为提交信息生成的额外指令。

```bash
# 推荐：使用引号
gcop-rs commit "用中文并保持简洁"

# 或不加引号（会被合并为一条指令）
gcop-rs commit 用中文 并 保持 简洁
```

> **注意**：在 JSON 模式（`--json` / `--format json`）下，gcop-rs 会以非交互方式运行，且**不会创建提交**（只输出 JSON）。

**交互式操作**:

生成信息后，你会看到一个菜单：

1. **Accept（接受）** - 使用生成的信息并创建提交
2. **Edit（编辑）** - 打开 `$VISUAL` / `$EDITOR`（未设置时使用系统默认编辑器）手动修改信息（编辑后返回菜单）
3. **Retry（重试）** - 不带额外指令重新生成新信息
4. **Retry with feedback（带反馈重试）** - 提供重新生成的指令（如 "用中文"、"更简洁"、"更详细"）。反馈会累积，多次重试可逐步优化结果
5. **Quit（退出）** - 取消提交过程

**示例**:

```bash
# 基本用法
git add src/auth.rs
gcop-rs commit

# 跳过所有提示
git add .
gcop-rs commit --no-edit --yes

# 使用不同的 provider
gcop-rs commit --provider openai

# 详细模式（查看 API 调用）
gcop-rs -v commit

# JSON 输出用于自动化（不会创建提交）
gcop-rs commit --json > commit.json
```

**工作流**:

```bash
$ git add src/auth.rs src/middleware.rs
$ gcop-rs commit

[1/4] 正在分析暂存的变更...
2 个文件已更改，45 处插入(+)，12 处删除(-)

ℹ 生成的提交信息:
feat(auth): 实现 JWT 令牌验证

添加用于验证 JWT 令牌的中间件，包含适当的
错误处理和过期检查。

[3/4] 选择下一步操作...
选择下一步操作:
> 接受
  编辑
  重试
  带反馈重试
  退出

[已选择: 接受]

[4/4] 正在创建提交...
✓ 提交创建成功！
```

**提示**:
- 运行前只暂存你想包含在此提交中的变更
- 在 CI/CD 流水线中使用 `--yes` 跳过交互式提示
- 使用 `--json` / `--format json` 生成提交信息用于脚本集成（不创建提交）
- 如果信息没有捕捉到你的意图，尝试"带反馈重试"

**输出格式 (json)**:

```json
{
  "success": true,
  "data": {
    "message": "feat(auth): 实现 JWT 令牌验证",
    "diff_stats": {
      "files_changed": ["src/auth.rs", "src/middleware.rs"],
      "insertions": 45,
      "deletions": 12,
      "total_changes": 57
    },
    "committed": false
  }
}
```

---

### review

对变更、提交或文件执行 AI 驱动的代码审查。

**语法**:
```bash
gcop-rs review [OPTIONS] <COMMAND>
```

**命令**:

| 命令 | 语法 | 说明 |
|------|------|------|
| 变更 | `gcop-rs review changes` | 审查工作区未暂存变更（类似 `git diff`） |
| 提交 | `gcop-rs review commit <HASH>` | 审查特定提交 |
| 范围 | `gcop-rs review range <RANGE>` | 审查提交范围（如 `HEAD~3..HEAD`） |
| 文件 | `gcop-rs review file <PATH>` | 审查单个文件（当前不支持目录） |

**选项**:

| 选项 | 说明 |
|------|------|
| `--format <FORMAT>`, `-f` | 输出格式: `text`（默认）、`json` 或 `markdown` |
| `--json` | `--format json` 的快捷方式 |
| `--provider <NAME>`, `-p` | 使用特定的 provider |

**示例**:

```bash
# 审查工作区变更（未暂存）
gcop-rs review changes

# 审查最后一次提交
gcop-rs review commit HEAD
gcop-rs review commit abc123

# 审查最近 3 次提交
gcop-rs review range HEAD~3..HEAD

# 审查单个文件
gcop-rs review file src/auth.rs

# 输出为 JSON 用于自动化
gcop-rs review changes --format json > review.json

# 输出为 markdown 用于文档
gcop-rs review changes --format markdown > REVIEW.md
```

> **注意**：当前 `review changes` 只会审查未暂存的变更（类似 `git diff`），不会包含已暂存的变更。
>
> **注意**：`review file <PATH>` 当前仅支持文件（不支持目录）。

**输出格式 (text)**:

```
ℹ 审查: 未提交的变更

📝 总结:
添加了 JWT 认证和适当的错误处理。
整体代码质量良好。

🔍 发现的问题:

  1. WARNING: 令牌刷新中缺少错误处理
     位置: src/auth.rs:45

  2. INFO: 考虑添加速率限制
     位置: src/middleware.rs:12

💡 建议:
  • 为边界情况添加单元测试
  • 记录令牌验证逻辑
  • 考虑将验证提取到单独的函数
```

**提示**:
- 提交前使用以尽早发现问题
- 使用 `--format json` 集成到 CI/CD
- 在配置中设置 `min_severity` 过滤噪音

---

### config

管理 gcop-rs 配置。

**语法**:
```bash
gcop-rs config [子命令]
```

不带子命令时，默认等同于 `gcop-rs config edit`。

**子命令**:

#### `config edit`

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

---

#### `config validate`

验证配置并测试 provider 连接。

**用法**:
```bash
gcop-rs config validate
```

**检查**:
- 加载并解析配置（默认值 + 配置文件 + `GCOP_*` 环境变量覆盖）
- 列出已配置的 providers
- 通过最小化测试请求验证 provider 连接（默认 provider + 配置的 `fallback_providers`）
- 只要至少有一个配置的 provider 验证成功就会返回成功

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

---

### alias

管理 gcop-rs 的 git 别名。

**语法**:
```bash
gcop-rs alias [OPTIONS]
```

**选项**:

| 选项 | 说明 |
|------|------|
| *(无)* | 安装所有别名（默认操作） |
| `--list`, `-l` | 列出所有可用的别名及其状态 |
| `--force`, `-f` | 强制安装，覆盖冲突 |
| `--remove`, `-r` | 删除别名（需要 `--force` 确认） |

**示例**:

#### 安装别名

```bash
# 安装所有 14 个别名
gcop-rs alias

# 输出:
[1/2] 正在安装 git 别名...
  ✓  git c          → AI 提交
  ✓  git r          → AI 审查
  ℹ  git p          → 推送 (已设置)

✓ 已安装 10 个别名
ℹ 已跳过 1 个别名（已存在或冲突）
```

#### 列出别名

```bash
gcop-rs alias --list

# 输出:
ℹ 可用的 git 别名:

  git cop        → 主入口                                [✓ 已安装]
  git c          → AI 提交                               [✓ 已安装]
  git r          → AI 审查                               [  未安装]
  git p          → 推送                                  [⚠ 冲突: !my-push]
  ...
```

#### 强制安装

```bash
# 覆盖冲突的别名
gcop-rs alias --force
```

#### 删除别名

```bash
# 预览将删除什么
gcop-rs alias --remove

# 输出:
⚠ 这将删除所有 gcop 相关的 git 别名

ℹ 将删除的别名:
  - git c
  - git r
  - git ac
  ...

ℹ 使用 --force 确认:
  gcop-rs alias --remove --force

# 实际删除
gcop-rs alias --remove --force
```

**何时使用**:
- 安装后：安装别名以获得便利
- gcop-rs 更新后：用 `--force` 重新安装
- 卸载时：用 `--remove --force` 删除

---

### stats

显示仓库提交统计。

**语法**:
```bash
gcop-rs stats [OPTIONS]
```

**说明**:

分析提交历史并显示统计信息，包括总提交数、贡献者、时间跨度和最近活动。

**选项**:

| 选项 | 说明 |
|------|------|
| `--format <FORMAT>`, `-f` | 输出格式: `text`（默认）、`json` 或 `markdown` |
| `--json` | `--format json` 的快捷方式 |
| `--author <NAME>` | 按作者名称或邮箱过滤统计 |

**示例**:

```bash
# 基本用法（文本格式）
gcop-rs stats

# 输出为 JSON 用于自动化
gcop-rs stats --format json
gcop-rs stats --json

# 输出为 Markdown 用于文档
gcop-rs stats --format markdown > STATS.md

# 按特定作者过滤
gcop-rs stats --author "john"
gcop-rs stats --author "john@example.com"
```

**输出格式 (text)**:

```
ℹ Repository Statistics
========================================

[] Overview
  Total commits:  156
  Contributors:   3
  Time span:      2024-06-15 ~ 2025-12-23 (192 days)

[] Top Contributors
  #1  AptS-1547 <esaps@esaps.net>  142 commits (91.0%)
  #2  bot <noreply@github.com>      8 commits  (5.1%)
  #3  contributor <x@y.com>         6 commits  (3.8%)

[] Recent Activity (last 4 weeks)
  2025-W52: ████████████           12
  2025-W51: ████████████████████   20
  2025-W50: ██████                  6
  2025-W49: ████████████████       16
```

**输出格式 (json)**:

```json
{
  "success": true,
  "data": {
    "total_commits": 156,
    "total_authors": 3,
    "first_commit_date": "2024-06-15T10:30:00+08:00",
    "last_commit_date": "2025-12-23T15:43:34+08:00",
    "authors": [
      {"name": "AptS-1547", "email": "esaps@esaps.net", "commits": 142},
      {"name": "bot", "email": "noreply@github.com", "commits": 8}
    ],
    "commits_by_week": {
      "2025-W49": 16,
      "2025-W50": 6,
      "2025-W51": 20,
      "2025-W52": 12
    }
  }
}
```

**提示**:
- 使用 `--format json` 集成到 CI/CD 或脚本
- 使用 `--author` 查看单个贡献者的统计
- ASCII 柱状图显示各周的相对活跃度

---

## 命令链接

gcop-rs 命令可以与标准 git 命令组合：

```bash
# 先审查，再暂存并提交
gcop-rs review changes && git add -A && gcop-rs commit

# 提交后推送（使用完整命令）
gcop-rs commit --yes && git push

# 或使用别名
git acp  # 等同于: add -A && commit && push
```

## 退出码

gcop-rs 使用标准退出码：

| 代码 | 含义 |
|------|------|
| 0 | 成功（在交互式菜单中取消也会返回 0） |
| 1 | 运行时错误（API 错误、git 错误、配置错误等） |
| 2 | 命令行用法错误（参数/选项无效，由 clap 返回） |

**在脚本中使用**:
```bash
if gcop-rs commit --yes; then
    echo "提交成功"
    git push
else
    echo "提交失败"
fi
```

## 环境变量

这些环境变量会影响 gcop-rs 行为：

| 变量 | 说明 |
|------|------|
| `ANTHROPIC_API_KEY` | Claude API key（如果不在配置中则作为回退） |
| `OPENAI_API_KEY` | OpenAI API key（回退） |
| `VISUAL` / `EDITOR` | commit message 编辑与 `gcop-rs config edit` 使用的编辑器 |
| `GCOP_UI_LANGUAGE` | 在启动早期强制指定 UI 语言（在完整加载配置前生效） |
| `GCOP_*` | 通过环境变量覆盖配置项（如 `GCOP_UI_COLORED=false`） |
| `NO_COLOR` | 禁用彩色输出（设置为任意值） |

**示例**:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export EDITOR="vim"
gcop-rs commit
```

## 参考

- [Git 别名指南](aliases.md) - Git 别名详细指南
- [配置参考](configuration.md) - 所有配置选项
- [Provider 设置](providers.md) - 配置 LLM providers
- [故障排除](troubleshooting.md) - 常见问题
