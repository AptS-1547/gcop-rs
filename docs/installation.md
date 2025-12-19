# Installation Guide

## System Requirements

- **Rust**: 1.70 or higher
- **Git**: Any recent version
- **Operating System**: Linux, macOS, or Windows

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

```bash
sudo cp target/release/gcop-rs /usr/local/bin/gcop-rs
```

**Option B: User installation**

```bash
mkdir -p ~/.local/bin
cp target/release/gcop-rs ~/.local/bin/gcop

# Add to PATH if not already (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.local/bin:$PATH"
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

```bash
cd gcop-rs
git pull
cargo build --release
sudo cp target/release/gcop-rs /usr/local/bin/gcop-rs
```

## Uninstall

```bash
# If installed to /usr/local/bin
sudo rm /usr/local/bin/gcop-rs

# If installed via cargo
cargo uninstall gcop-rs

# Remove config (optional)
rm -rf ~/.config/gcop
```

## Next Steps

1. [Configure your LLM provider](configuration.md)
2. Try the [basic usage](../README.md#quick-start)
3. Explore [advanced features](providers.md)
