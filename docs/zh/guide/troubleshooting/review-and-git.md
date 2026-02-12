# 代码审查问题

## 问题: "Failed to parse review result"

**原因**: LLM 没有返回有效的 JSON

**解决方案**:

1. **使用详细模式**开启调试日志：
   ```bash
   gcop-rs -v review changes
   ```

   这样可以在日志中看到更多解析上下文（例如错误里的响应预览）；不过 review 不会打印完整 prompt/响应正文。

2. **检查自定义 prompt**（如果使用）：
   - 确保明确要求 JSON 格式
   - 提供准确的 JSON schema 示例

3. **尝试不同模型**：
   ```bash
   # 某些模型处理 JSON 更好
   gcop-rs --provider openai review changes
   ```

4. **调整 temperature**：
   ```toml
   temperature = 0.1  # 更低 = 更一致的输出
   ```

## Git 问题

## 问题: "No staged changes found"

**原因**: Git 暂存区为空

**解决方案**:
```bash
# 先暂存变更
git add <files>

# 或暂存所有变更
git add .

# 然后运行 gcop
gcop-rs commit
```

## 问题: "Not a git repository"

**原因**: 当前目录不是 git 仓库

**解决方案**:
```bash
# 初始化 git 仓库
git init

# 或在 git 仓库中运行 gcop
cd /path/to/your/git/repo
```

