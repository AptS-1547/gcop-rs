# 安装指南

## 系统要求

- **Rust**: 1.70 或更高版本
- **Git**: 任意最新版本
- **操作系统**: Linux、macOS 或 Windows

## 从源码安装

### 1. 克隆仓库

```bash
git clone https://github.com/AptS-1547/gcop-rs.git
cd gcop-rs
```

### 2. 编译

```bash
# Release 编译（优化版本）
cargo build --release

# 开发编译（编译更快）
cargo build
```

二进制文件位置：
- Release: `target/release/gcop-rs`
- Debug: `target/debug/gcop-rs`

### 3. 安装

**选项 A: 系统全局安装**

```bash
sudo cp target/release/gcop-rs /usr/local/bin/gcop-rs
```

**选项 B: 用户安装**

```bash
mkdir -p ~/.local/bin
cp target/release/gcop-rs ~/.local/bin/gcop

# 添加到 PATH（如果还没有，添加到 ~/.bashrc 或 ~/.zshrc）
export PATH="$HOME/.local/bin:$PATH"
```

**选项 C: 使用 Cargo 安装**

```bash
cargo install --path .
```

### 4. 验证安装

```bash
gcop-rs --version
# 应该输出: gcop-rs 0.1.0

gcop-rs --help
# 应该显示帮助信息
```

## 更新

```bash
cd gcop-rs
git pull
cargo build --release
sudo cp target/release/gcop-rs /usr/local/bin/gcop-rs
```

## 卸载

```bash
# 如果安装到 /usr/local/bin
sudo rm /usr/local/bin/gcop-rs

# 如果通过 cargo 安装
cargo uninstall gcop-rs

# 删除配置（可选）
rm -rf ~/.config/gcop
```

## 下一步

1. [配置 LLM 提供商](configuration.md)
2. 尝试[基本使用](../README_ZH.md#快速开始)
3. 探索[高级功能](providers.md)
