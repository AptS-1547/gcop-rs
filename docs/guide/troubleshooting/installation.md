# Installation Issues

## Issue: `cargo build` fails

**Solution**:
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

## Issue: Binary not found after install

**Solution**:
```bash
# Check if binary exists
ls -la /usr/local/bin/gcop-rs

# Verify PATH includes /usr/local/bin
echo $PATH

# Add to PATH if needed
export PATH="/usr/local/bin:$PATH"
```

