# 安装指南

## 系统要求

- **Rust**: 1.70 或更高版本
- **Git**: 任意最新版本
- **操作系统**: Linux、macOS 或 Windows

## 快速安装（推荐）

### Homebrew (macOS/Linux)

```bash
brew tap AptS-1547/gcop-rs
brew install gcop-rs
```

支持 macOS (Intel/Apple Silicon) 和 Linux (x86_64/ARM64)。

### pipx / pip (Python 用户)

```bash
# 使用 pipx（推荐用于 CLI 工具）
pipx install gcop-rs

# 使用 pip
pip install gcop-rs
```

这是一个 Python 包装器，首次运行时会自动下载预编译的 Rust 二进制文件。

### cargo-binstall

如果你安装了 [cargo-binstall](https://github.com/cargo-bins/cargo-binstall)：

```bash
cargo binstall gcop-rs
```

直接下载预编译二进制，无需编译。

### cargo install

```bash
cargo install gcop-rs
```

这将从 [crates.io](https://crates.io/crates/gcop-rs) 下载并编译。

### 预编译二进制

从 [GitHub Releases](https://github.com/AptS-1547/gcop-rs/releases) 下载：

| 平台 | 文件 |
|------|------|
| macOS (Apple Silicon) | `gcop-rs-vX.X.X-macos-arm64` |
| macOS (Intel) | `gcop-rs-vX.X.X-macos-amd64` |
| Linux (x86_64) | `gcop-rs-vX.X.X-linux-amd64` |
| Linux (ARM64) | `gcop-rs-vX.X.X-linux-arm64` |
| Windows (x86_64) | `gcop-rs-vX.X.X-windows-amd64.exe` |
| Windows (ARM64) | `gcop-rs-vX.X.X-windows-aarch64.exe` |

### 验证安装

```bash
gcop-rs --version
gcop-rs --help
```

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

Linux/macOS:
```bash
sudo cp target/release/gcop-rs /usr/local/bin/gcop-rs
```

Windows (以管理员身份运行 PowerShell):
```powershell
Copy-Item target\release\gcop-rs.exe C:\Windows\System32\gcop-rs.exe
```

**选项 B: 用户安装**

Linux/macOS:
```bash
mkdir -p ~/.local/bin
cp target/release/gcop-rs ~/.local/bin/gcop-rs

# 添加到 PATH（如果还没有，添加到 ~/.bashrc 或 ~/.zshrc）
export PATH="$HOME/.local/bin:$PATH"
```

Windows (PowerShell):
```powershell
# 创建目录（如果不存在）
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.local\bin"

# 复制二进制文件
Copy-Item target\release\gcop-rs.exe "$env:USERPROFILE\.local\bin\gcop-rs.exe"

# 添加到 PATH（运行一次，然后重启终端）
[Environment]::SetEnvironmentVariable("Path", "$env:Path;$env:USERPROFILE\.local\bin", "User")
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

**如果通过 cargo 安装：**

```bash
cargo install gcop-rs --force
```

**如果从源码安装：**

Linux/macOS:
```bash
cd gcop-rs
git pull
cargo build --release
sudo cp target/release/gcop-rs /usr/local/bin/gcop-rs
```

Windows (以管理员身份运行 PowerShell):
```powershell
cd gcop-rs
git pull
cargo build --release
Copy-Item target\release\gcop-rs.exe C:\Windows\System32\gcop-rs.exe
```

## 卸载

**如果通过 cargo 安装：**

```bash
cargo uninstall gcop-rs
```

**如果手动安装：**

Linux/macOS:
```bash
# 如果安装到 /usr/local/bin
sudo rm /usr/local/bin/gcop-rs

# 如果安装到 ~/.local/bin
rm ~/.local/bin/gcop-rs
```

Windows (以管理员身份运行 PowerShell):
```powershell
# 如果安装到 System32
Remove-Item C:\Windows\System32\gcop-rs.exe

# 如果安装到用户目录
Remove-Item "$env:USERPROFILE\.local\bin\gcop-rs.exe"
```

**删除配置（可选）：**

Linux/macOS:
```bash
rm -rf ~/.config/gcop
```

Windows (PowerShell):
```powershell
Remove-Item -Recurse -Force "$env:APPDATA\gcop"
```

## 下一步

1. [配置 LLM 提供商](configuration.md)
2. 尝试[基本使用](../README_ZH.md#快速开始)
3. 探索[高级功能](providers.md)
