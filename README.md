# gcop-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/gcop-rs)](https://crates.io/crates/gcop-rs)
[![Downloads](https://img.shields.io/crates/d/gcop-rs)](https://crates.io/crates/gcop-rs)
[![CI](https://github.com/AptS-1547/gcop-rs/workflows/CI/badge.svg)](https://github.com/AptS-1547/gcop-rs/actions)

AI-powered Git CLI — generate commit messages, review code, manage workflows, all from your terminal. Written in Rust.

> Rust rewrite inspired by [gcop](https://github.com/Undertone0809/gcop) by [Undertone0809](https://github.com/Undertone0809).

**[中文文档](README_ZH.md)** | **[Documentation](https://gcop.docs.esap.cc/)**

## Features

- **AI Commit Messages** — Generate conventional commits via Claude, OpenAI, Gemini, or Ollama
- **Code Review** — AI-powered review with security & performance insights
- **Monorepo Support** — Auto-detect Cargo, Pnpm, Npm, Lerna, Nx, Turbo workspaces and infer commit scope
- **Git Aliases** — Shortcuts like `git c`, `git r`, `git acp` for streamlined workflow
- **Git Hook** — `prepare-commit-msg` hook for seamless editor integration
- **Custom Providers** — Any OpenAI/Claude-compatible API (DeepSeek, custom endpoints, etc.)
- **Custom Prompts** — Template variables for commit & review prompt customization
- **Project Config** — Per-repo `.gcop/config.toml` overrides user config
- **GPG Signing** — Full support via native git
- **Beautiful CLI** — Spinner animations, streaming output, colored text, interactive menus

## Quick Start

### 1. Install

```bash
# Homebrew (macOS/Linux)
brew tap AptS-1547/tap
brew install gcop-rs

# pipx (Python users)
pipx install gcop-rs

# cargo-binstall (prebuilt binary, no compilation)
cargo binstall gcop-rs

# cargo install (from source)
cargo install gcop-rs
```

See [Installation Guide](https://gcop.docs.esap.cc/guide/installation) for more options.

### 2. Configure

```bash
gcop-rs init
```

The interactive wizard creates your config file and optionally installs git aliases.

Or set up manually — use `gcop-rs config edit` to open your config in the system editor:

```toml
[llm]
default_provider = "claude"

[llm.providers.claude]
api_key = "sk-ant-your-key-here"
model = "claude-sonnet-4-5-20250929"
```

Config locations: `~/.config/gcop/` (Linux), `~/Library/Application Support/gcop/` (macOS), `%APPDATA%\gcop\config\` (Windows).

Environment overrides: `GCOP__LLM__PROVIDERS__CLAUDE__API_KEY`, etc. See [Configuration Guide](https://gcop.docs.esap.cc/guide/configuration).

### 3. Use

```bash
git add .
gcop-rs commit            # Generate AI commit message → review → commit
gcop-rs review changes    # AI review of working tree changes

# Or with aliases (after gcop-rs alias):
git c                     # = gcop-rs commit
git acp                   # Add all → AI commit → push
```

The commit workflow is interactive — after generation, you can **accept**, **edit**, **retry**, or **retry with feedback** (e.g. "use Chinese", "be more concise") to refine the result.

## Commands

| Command | Description |
|---------|-------------|
| `gcop-rs commit` | Generate AI commit message for staged changes |
| `gcop-rs review <target>` | Review `changes` / `commit <hash>` / `range <a..b>` / `file <path>` |
| `gcop-rs init` | Interactive configuration setup |
| `gcop-rs config edit` | Edit config with post-save validation |
| `gcop-rs config validate` | Validate config & test provider connection |
| `gcop-rs alias` | Install / list / remove git aliases |
| `gcop-rs stats` | Repository commit statistics |
| `gcop-rs hook install` | Install `prepare-commit-msg` hook |
| `gcop-rs hook uninstall` | Remove the hook |

Global flags: `-v` verbose, `--provider <name>` override, `--format text|json|markdown`, `--dry-run`.

See [Command Reference](https://gcop.docs.esap.cc/guide/commands) for full details.

## Git Aliases

Install with `gcop-rs alias` or during `gcop-rs init`.

| Alias | Action |
|-------|--------|
| `git c` | AI commit |
| `git r` | AI review changes |
| `git s` | Repository stats |
| `git ac` | Add all + AI commit |
| `git cp` | AI commit + push |
| `git acp` | Add all + AI commit + push |
| `git gconfig` | Edit gcop-rs config |
| `git p` | Push |
| `git pf` | Force push (`--force-with-lease`) |
| `git undo` | Undo last commit (keep staged) |

Manage: `--list`, `--force`, `--remove --force`. See [Aliases Guide](https://gcop.docs.esap.cc/guide/aliases).

## Documentation

- [Installation](https://gcop.docs.esap.cc/guide/installation) — All installation methods
- [Configuration](https://gcop.docs.esap.cc/guide/configuration) — Complete config reference
- [Commands](https://gcop.docs.esap.cc/guide/commands) — Detailed command docs
- [Providers](https://gcop.docs.esap.cc/guide/providers) — Provider setup (Claude, OpenAI, Gemini, Ollama, custom)
- [Custom Prompts](https://gcop.docs.esap.cc/guide/prompts) — Template variables and examples
- [Git Aliases](https://gcop.docs.esap.cc/guide/aliases) — Full aliases reference
- [Troubleshooting](https://gcop.docs.esap.cc/guide/troubleshooting) — Common issues and solutions

## Requirements

- **Git** 2.0+
- **API key** for at least one provider (Claude, OpenAI, Gemini), or local [Ollama](https://ollama.ai)
- **Rust** 1.88.0+ (only if building from source)

## License

MIT — see [LICENSE](LICENSE).

## Credits

This project is a Rust rewrite inspired by [gcop](https://github.com/Undertone0809/gcop) by **[Undertone0809](https://github.com/Undertone0809)**. The core concept of AI-powered commit message generation originated from that project.

**Authors**: [AptS-1547](https://github.com/AptS-1547), [AptS-1738](https://github.com/AptS-1738), [uaih3k9x](https://github.com/uaih3k9x)
