# Code Review Issues

## Issue: "Failed to parse review result"

**Cause**: LLM didn't return valid JSON

**Solution**:

1. **Use verbose mode** to enable debug logs:
   ```bash
   gcop-rs -v review changes
   ```

   This helps surface parsing context in logs (for example response preview in errors), even though full prompt/response bodies are not printed for review.

2. **Check your custom prompt** (if using one):
   - Ensure it explicitly requests JSON format
   - Provide exact JSON schema example

3. **Try different model**:
   ```bash
   # Some models handle JSON better
   gcop-rs --provider openai review changes
   ```

4. **Adjust temperature**:
   ```toml
   temperature = 0.1  # Lower = more consistent output
   ```

## Git Issues

## Issue: "No staged changes found"

**Cause**: Nothing added to git staging area

**Solution**:
```bash
# Stage your changes first
git add <files>

# Or stage all changes
git add .

# Then run gcop
gcop-rs commit
```

## Issue: "Not a git repository"

**Cause**: Current directory is not a git repo

**Solution**:
```bash
# Initialize git repository
git init

# Or run gcop from within a git repository
cd /path/to/your/git/repo
```

