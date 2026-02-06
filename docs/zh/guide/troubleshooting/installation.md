# 安装问题

## 问题: `cargo build` 失败

**解决方案**:
```bash
# 更新 Rust
rustup update

# 清理并重新编译
cargo clean
cargo build --release
```

## 问题: 安装后找不到二进制文件

**解决方案**:
```bash
# 检查二进制文件是否存在
ls -la /usr/local/bin/gcop-rs

# 验证 PATH 包含 /usr/local/bin
echo $PATH

# 如需要添加到 PATH
export PATH="/usr/local/bin:$PATH"
```

