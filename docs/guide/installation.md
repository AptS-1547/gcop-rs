# Installation Guide

## System Requirements

- **Rust**: 1.92.0 or higher (Rust 2024 edition)
- **Git**: 2.0 or higher
- **Operating System**: Linux, macOS, or Windows

## Quick Install (Recommended)

### Homebrew (macOS/Linux)

```bash
brew tap AptS-1547/gcop-rs
brew install gcop-rs
```

Supports macOS (Intel/Apple Silicon) and Linux (x86_64/ARM64).

### pipx / pip (Python users)

```bash
# Using pipx (recommended for CLI tools)
pipx install gcop-rs

# Using pip
pip install gcop-rs
```

Pre-compiled binaries are bundled in platform-specific wheels. No additional downloads required after installation.

### cargo-binstall

If you have [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) installed:

```bash
cargo binstall gcop-rs
```

This downloads pre-compiled binaries without compilation.

### cargo install

```bash
cargo install gcop-rs
```

This will download and compile from [crates.io](https://crates.io/crates/gcop-rs).

### Pre-compiled Binaries

Download from [GitHub Releases](https://github.com/AptS-1547/gcop-rs/releases):

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `gcop-rs-vX.X.X-macos-arm64` |
| macOS (Intel) | `gcop-rs-vX.X.X-macos-amd64` |
| Linux (x86_64) | `gcop-rs-vX.X.X-linux-amd64` |
| Linux (ARM64) | `gcop-rs-vX.X.X-linux-arm64` |
| Windows (x86_64) | `gcop-rs-vX.X.X-windows-amd64.exe` |
| Windows (ARM64) | `gcop-rs-vX.X.X-windows-aarch64.exe` |

### Verify Installation

```bash
gcop-rs --version
gcop-rs --help
```

## From Source

### 1. Clone Repository

```bash
git clone https://github.com/AptS-1547/gcop-rs.git
cd gcop-rs
```

### 2. Build

```bash
# Release build (optimized)
cargo build --release

# Development build (faster compilation)
cargo build
```

The binary will be at:
- Release: `target/release/gcop-rs`
- Debug: `target/debug/gcop-rs`

### 3. Install

**Option A: System-wide installation**

Linux/macOS:
```bash
sudo cp target/release/gcop-rs /usr/local/bin/gcop-rs
```

Windows (PowerShell as Administrator):
```powershell
Copy-Item target\release\gcop-rs.exe C:\Windows\System32\gcop-rs.exe
```

**Option B: User installation**

Linux/macOS:
```bash
mkdir -p ~/.local/bin
cp target/release/gcop-rs ~/.local/bin/gcop-rs

# Add to PATH if not already (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.local/bin:$PATH"
```

Windows (PowerShell):
```powershell
# Create directory if not exists
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.local\bin"

# Copy binary
Copy-Item target\release\gcop-rs.exe "$env:USERPROFILE\.local\bin\gcop-rs.exe"

# Add to PATH (run once, then restart terminal)
[Environment]::SetEnvironmentVariable("Path", "$env:Path;$env:USERPROFILE\.local\bin", "User")
```

**Option C: Cargo install**

```bash
cargo install --path .
```

### 4. Verify Installation

```bash
gcop-rs --version
# Should output: gcop-rs 0.1.0

gcop-rs --help
# Should show help information
```

## Update

**If installed via Homebrew:**

```bash
brew upgrade gcop-rs
```

**If installed via pipx:**

```bash
pipx upgrade gcop-rs
```

**If installed via pip:**

```bash
pip install --upgrade gcop-rs
```

**If installed via cargo-binstall or cargo:**

```bash
cargo install gcop-rs --force
```

**If installed from source:**

Linux/macOS:

```bash
cd gcop-rs
git pull
cargo build --release
sudo cp target/release/gcop-rs /usr/local/bin/gcop-rs
```

Windows (PowerShell as Administrator):

```powershell
cd gcop-rs
git pull
cargo build --release
Copy-Item target\release\gcop-rs.exe C:\Windows\System32\gcop-rs.exe
```

## Uninstall

**If installed via Homebrew:**

```bash
brew uninstall gcop-rs
brew untap AptS-1547/gcop-rs  # Optional, remove the tap
```

**If installed via pipx:**

```bash
pipx uninstall gcop-rs
```

**If installed via pip:**

```bash
pip uninstall gcop-rs
```

**If installed via cargo-binstall or cargo:**

```bash
cargo uninstall gcop-rs
```

**If installed manually:**

Linux/macOS:

```bash
# If installed to /usr/local/bin
sudo rm /usr/local/bin/gcop-rs

# If installed to ~/.local/bin
rm ~/.local/bin/gcop-rs
```

Windows (PowerShell as Administrator):

```powershell
# If installed to System32
Remove-Item C:\Windows\System32\gcop-rs.exe

# If installed to user directory
Remove-Item "$env:USERPROFILE\.local\bin\gcop-rs.exe"
```

**Remove config (optional):**

Linux/macOS:

```bash
rm -rf ~/.config/gcop
```

Windows (PowerShell):

```powershell
Remove-Item -Recurse -Force "$env:APPDATA\gcop"
```

## Next Steps

1. [Configure your LLM provider](configuration.md)
2. Try the [basic usage](commands.md)
3. Explore [advanced features](providers.md)
