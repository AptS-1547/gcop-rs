# gcop-rs

[![PyPI](https://img.shields.io/pypi/v/gcop-rs)](https://pypi.org/project/gcop-rs/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

AI-powered Git commit message generator and code reviewer, written in Rust.

This is a Python wrapper that automatically downloads and runs the pre-compiled Rust binary.

## Installation

```bash
# Using pipx (recommended)
pipx install gcop-rs

# Using pip
pip install gcop-rs
```

## Usage

```bash
# Generate commit message
gcop-rs commit

# Code review
gcop-rs review

# Show help
gcop-rs --help
```

## Other Installation Methods

For native installation without Python:

```bash
# Homebrew (macOS/Linux)
brew tap AptS-1547/gcop-rs
brew install gcop-rs

# cargo-binstall
cargo binstall gcop-rs

# cargo install
cargo install gcop-rs
```

## Documentation

See the [main repository](https://github.com/AptS-1547/gcop-rs) for full documentation.

## License

MIT License
