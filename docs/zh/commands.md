# 命令参考

所有 gcop-rs 命令和选项的完整参考。

## 全局选项

这些选项可以用于任何命令：

| 选项 | 说明 |
|------|------|
| `--provider <NAME>` | 覆盖默认 LLM provider (claude, openai, ollama 或自定义) |
| `--verbose`, `-v` | 启用详细日志（显示 API 请求和响应） |
| `--help`, `-h` | 显示帮助信息 |
| `--version`, `-V` | 显示版本信息 |

**示例**:
```bash
gcop-rs --provider openai commit
gcop-rs -v review
```

---

## 命令

### init

使用交互式向导初始化 gcop-rs 配置。

**语法**:
```bash
gcop-rs init
```

**说明**:

交互式设置，指导你完成：
1. 创建配置目录 (`~/.config/gcop/`)
2. 复制示例配置文件
3. 设置安全文件权限 (600)
4. 可选安装 git 别名

**选项**: 无

**示例**:
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

✓ 已安装 11 个别名
```

**创建的内容**:
- `~/.config/gcop/config.toml` (来自 `examples/config.toml.example`)
- Git 别名配置到 `~/.gitconfig`（如果选择安装）

**何时使用**: 首次设置或从头重新配置时。

---

### commit

生成 AI 驱动的提交信息并创建提交。

**语法**:
```bash
gcop-rs commit [OPTIONS]
```

**说明**:

分析暂存的变更，使用 AI 生成符合规范的提交信息，并在你批准后创建 git 提交。

**选项**:

| 选项 | 说明 |
|------|------|
| `--no-edit` | 跳过打开编辑器手动编辑 |
| `--yes` | 跳过确认菜单并接受生成的信息 |
| `--provider <NAME>` | 使用特定的 provider（覆盖默认值） |

**交互式操作**:

生成信息后，你会看到一个菜单：

1. **Accept（接受）** - 使用生成的信息并创建提交
2. **Edit（编辑）** - 打开 `$EDITOR` 手动修改信息
3. **Retry（重试）** - 不带额外指令重新生成新信息
4. **Retry with feedback（带反馈重试）** - 提供重新生成的指令（如 "用中文"、"更简洁"、"更详细"）
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
- 如果信息没有捕捉到你的意图，尝试"带反馈重试"

---

### review

对变更、提交或文件执行 AI 驱动的代码审查。

**语法**:
```bash
gcop-rs review [TARGET] [OPTIONS]
```

**目标**:

| 目标 | 语法 | 说明 |
|------|------|------|
| *(默认)* | `gcop-rs review` | 审查未提交的变更 |
| 提交 | `--commit <HASH>` | 审查特定提交 |
| 范围 | `--range <RANGE>` | 审查提交范围（如 `HEAD~3..HEAD`） |
| 文件 | `--file <PATH>` | 审查特定文件 |

**选项**:

| 选项 | 说明 |
|------|------|
| `--format <FORMAT>` | 输出格式: `text`（默认）、`json` 或 `markdown` |
| `--provider <NAME>` | 使用特定的 provider |

**示例**:

```bash
# 审查未提交的变更（默认）
gcop-rs review

# 审查最后一次提交
gcop-rs review --commit HEAD
gcop-rs review --commit abc123

# 审查最近 3 次提交
gcop-rs review --range HEAD~3..HEAD

# 审查特定文件
gcop-rs review --file src/auth.rs

# 输出为 JSON 用于自动化
gcop-rs review --format json > review.json

# 输出为 markdown 用于文档
gcop-rs review --format markdown > REVIEW.md
```

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
gcop-rs config <子命令>
```

**子命令**:

#### `config edit`

在默认编辑器中打开配置文件。

**用法**:
```bash
gcop-rs config edit
```

**打开**: 在 `$EDITOR` 中打开 `~/.config/gcop/config.toml`（回退到 `vi`）

**何时使用**: 修改 API keys、模型或自定义 prompts。

---

#### `config validate`

验证配置并测试 provider 连接。

**用法**:
```bash
gcop-rs config validate
```

**检查**:
- 配置文件语法
- 必需字段是否存在
- API key 格式
- Provider 连接性（进行测试 API 调用）

**示例输出**:
```
✓ 配置文件有效
✓ Claude provider 验证成功
⚠ OpenAI provider: API key 未设置（已跳过）
```

**何时使用**:
- 编辑配置后
- 排查连接问题
- 验证 API keys

---

#### `config show`

显示当前配置。

**用法**:
```bash
gcop-rs config show
```

**示例输出**:
```toml
[llm]
default_provider = "claude"

[llm.providers.claude]
model = "claude-sonnet-4-5-20250929"
endpoint = "https://api.anthropic.com/v1/messages"

[commit]
show_diff_preview = true
allow_edit = true

[ui]
colored = true
```

**注意**: 出于安全考虑，API keys 会被隐藏。

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
| `--list` | 列出所有可用的别名及其状态 |
| `--force` | 强制安装，覆盖冲突 |
| `--remove` | 删除别名（需要 `--force` 确认） |

**示例**:

#### 安装别名

```bash
# 安装所有 11 个别名
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

## 命令链接

gcop-rs 命令可以与标准 git 命令组合：

```bash
# 审查后提交
gcop-rs review && gcop-rs commit

# 提交后推送（使用完整命令）
gcop-rs commit --yes && git push

# 或使用别名
git acp  # 等同于: add -A && commit && push
```

## 退出码

gcop-rs 使用标准退出码：

| 代码 | 含义 |
|------|------|
| 0 | 成功 |
| 1 | 一般错误（API 错误、git 错误等） |
| 2 | 用户取消（Ctrl+C 或选择退出） |
| 3 | 配置错误 |
| 4 | 无效输入（无变更、无效的 commit hash 等） |

**在脚本中使用**:
```bash
if gcop-rs commit --yes; then
    echo "提交成功"
    git push
else
    echo "提交失败或取消"
fi
```

## 环境变量

这些环境变量会影响 gcop-rs 行为：

| 变量 | 说明 |
|------|------|
| `ANTHROPIC_API_KEY` | Claude API key（如果不在配置中则作为回退） |
| `OPENAI_API_KEY` | OpenAI API key（回退） |
| `EDITOR` | 用于 `--edit` 和 `config edit` 的编辑器 |
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
