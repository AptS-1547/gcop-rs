# gcop-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/gcop-rs)](https://crates.io/crates/gcop-rs)
[![Downloads](https://img.shields.io/crates/d/gcop-rs)](https://crates.io/crates/gcop-rs)
[![CI](https://github.com/AptS-1547/gcop-rs/workflows/CI/badge.svg)](https://github.com/AptS-1547/gcop-rs/actions)

AI-powered Git commit message generator and code reviewer, written in Rust.

> **Note**: This is a Rust rewrite of the original [gcop](https://github.com/Undertone0809/gcop) project (Python). Since the original project is no longer actively maintained, this version was created to provide better performance, reliability, and maintainability.

**[中文文档](README_ZH.md)** | **[Documentation](docs/)**

## Features

- 🤖 **AI Commit Messages** - Generate conventional commit messages using Claude, OpenAI, or Ollama
- 🔍 **Code Review** - Get AI-powered code reviews with security and performance insights
- 🎯 **Git Aliases** - Convenient shortcuts like `git c`, `git r`, `git acp` for streamlined workflow
- 🚀 **Easy Setup** - Interactive `init` command for quick configuration
- 🔧 **Custom Providers** - Support any OpenAI/Claude compatible API (DeepSeek, custom endpoints, etc.)
- 📝 **Custom Prompts** - Customize generation and review prompts with template variables
- ⚙️  **Flexible Config** - Configure via file or environment variables
- 🎨 **Beautiful CLI** - Spinner animations, colored output, and interactive prompts
- 🐛 **Debug Mode** - Verbose logging with full request/response inspection
- 🔐 **GPG Signing** - Full support for GPG commit signing via native git

## Quick Start

### 1. Installation

```bash
# Clone and build
git clone https://github.com/AptS-1547/gcop-rs.git
cd gcop-rs
cargo build --release

# Copy to PATH
sudo cp target/release/gcop-rs /usr/local/bin/gcop-rs
```

See [docs/installation.md](docs/installation.md) for more options.

### 2. Configure

**Option 1: Quick setup (recommended)**

```bash
gcop-rs init
```

This interactive wizard will:
- Create config directory and file
- Set secure file permissions (chmod 600)
- Optionally install convenient git aliases

**Option 2: Manual setup**

Create `~/.config/gcop/config.toml`:

```toml
[llm]
default_provider = "claude"

[llm.providers.claude]
api_key = "sk-ant-your-key-here"
model = "claude-sonnet-4-5-20250929"
```

Or use environment variables:
```bash
export ANTHROPIC_API_KEY="sk-ant-your-key"
```

See [docs/configuration.md](docs/configuration.md) for all options.

### 3. Use

```bash
# Generate commit message
git add .
gcop-rs commit
# Or use the alias: git c

# Review uncommitted changes
gcop-rs review
# Or use the alias: git r

# Complete workflow
git acp  # Add all, commit with AI, and push

# Use different provider
gcop-rs --provider openai commit
```

## Git Aliases

gcop-rs provides convenient git aliases for streamlined workflow.

### Installation

```bash
# Install all aliases
gcop-rs alias

# Or during initial setup
gcop-rs init  # Will prompt to install aliases
```

### Usage

After installation, you can use these shortcuts:

```bash
git c          # AI commit message and commit
git r          # AI review uncommitted changes
git ac         # Add all changes and commit with AI
git acp        # Add all, commit with AI, and push
git gconfig    # Edit gcop-rs configuration
git p          # Push to remote
git pf         # Force push (safer with --force-with-lease)
git undo       # Undo last commit (keep changes staged)
```

### Management

```bash
# List all available aliases
gcop-rs alias --list

# Reinstall (overwrite conflicts)
gcop-rs alias --force

# Remove all gcop-rs aliases
gcop-rs alias --remove --force
```

See [docs/aliases.md](docs/aliases.md) for detailed information on each alias.

## Commands

### `gcop-rs init`

Initialize gcop-rs configuration.

```bash
gcop-rs init
```

Interactive setup wizard that:
- Creates config directory
- Copies example configuration
- Sets secure file permissions
- Optionally installs git aliases

---

### `gcop-rs commit`

Generate AI-powered commit message for staged changes.

```bash
gcop-rs commit              # Generate, review, and commit
gcop-rs commit --no-edit    # Skip editor
gcop-rs commit --yes        # Skip confirmation
gcop-rs -v commit           # Verbose mode
```

**Interactive workflow**:

After generating a commit message, you can choose:
- **Accept** - Use the generated message
- **Edit** - Open editor to manually modify (returns to menu after editing)
- **Retry** - Regenerate without feedback
- **Retry with feedback** - Provide custom instructions (e.g., "use Chinese", "be more concise"). Feedback accumulates across retries for refined results
- **Quit** - Cancel commit

Example:
```bash
$ git add .
$ gcop-rs commit

ℹ Generated commit message:
feat(auth): implement JWT token validation

Choose next action:
> Accept
  Edit
  Retry
  Retry with feedback
  Quit
```

---

### `gcop-rs review`

Review code changes with AI.

```bash
gcop-rs review                   # Review uncommitted changes
gcop-rs review --commit <hash>   # Review a commit
gcop-rs review --range main..dev # Review commit range
gcop-rs review --file src/main.rs # Review a file
```

**Output formats**: `--format text|json|markdown`

---

### `gcop-rs config`

Manage configuration.

```bash
# Edit config file in your default editor (with validation)
gcop-rs config edit

# Validate configuration and test provider connection
gcop-rs config validate

# Show current configuration
gcop-rs config show
```

`config edit` validates your config after saving (like `visudo`) and works even when config is corrupted.

> **Tip**: Always use `gcop-rs config edit` instead of editing the config file directly to avoid syntax errors.

---

### `gcop-rs alias`

Manage git aliases.

```bash
gcop-rs alias                       # Install all aliases
gcop-rs alias --list                # List available aliases
gcop-rs alias --force               # Overwrite conflicts
gcop-rs alias --remove --force      # Remove all aliases
```

Provides convenient shortcuts like `git c`, `git r`, `git acp`, etc.

See [docs/aliases.md](docs/aliases.md) for details.

## Configuration

Config file location: `~/.config/gcop/config.toml`

Example configuration with Claude API:

```toml
[llm]
default_provider = "claude"

[llm.providers.claude]
api_key = "sk-ant-your-key"
model = "claude-sonnet-4-5-20250929"
temperature = 0.3

[commit]
show_diff_preview = true
allow_edit = true

[review]
min_severity = "info"

[ui]
colored = true
```

For complete configuration reference, see [docs/configuration.md](docs/configuration.md).

## Advanced Features

### Custom Providers

Add any OpenAI or Claude compatible API:

```toml
[llm.providers.deepseek]
api_style = "openai"
api_key = "sk-your-deepseek-key"
endpoint = "https://api.deepseek.com/v1/chat/completions"
model = "deepseek-chat"
```

See [docs/providers.md](docs/providers.md) for more examples.

### Custom Prompts

Customize commit message or review prompts:

```toml
[commit]
custom_prompt = """
Generate a commit message in Chinese for:
{diff}

Files: {files_changed}
Stats: +{insertions} -{deletions}
"""
```

See [docs/prompts.md](docs/prompts.md) for template variables and examples.

### Debug Mode

Use `--verbose` to see detailed logs:

```bash
gcop-rs -v commit  # Shows API requests, responses, and prompts
```

## Documentation

- **[Installation Guide](docs/installation.md)** - Detailed installation instructions
- **[Git Aliases Guide](docs/aliases.md)** - Complete guide to git aliases
- **[Command Reference](docs/commands.md)** - Detailed command documentation
- **[Configuration Reference](docs/configuration.md)** - Complete configuration guide
- **[Provider Setup](docs/providers.md)** - Configure LLM providers
- **[Custom Prompts](docs/prompts.md)** - Customize AI prompts
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions

## Requirements

- Rust 1.70 or higher
- Git
- API key for at least one provider (Claude, OpenAI, or local Ollama)

## License

MIT License - see LICENSE file for details.

## Author

AptS:1547 (Yuhan Bian / 卞雨涵) <apts-1547@esaps.net>

---

**Tip**: Run `gcop-rs --help` to see all commands, or use `git c` after installing aliases for quick commits!
