//! # gcop-rs
//!
//! AI 驱动的 Git 工具，用于生成 commit message 和代码审查。
//!
//! 这是原 Python 项目 [gcop](https://github.com/Undertone0809/gcop) 的 Rust 重写版本。
//!
//! ## 功能
//! - **Commit message 生成**：基于 staged changes 自动生成符合 Conventional Commits 规范的 commit message
//! - **代码审查**：分析代码变更，识别潜在问题和改进建议
//! - **多 Provider 支持**：Claude, OpenAI, Gemini, Ollama（本地模型）
//! - **高可用**：Fallback 机制，主 provider 失败时自动切换
//! - **流式输出**：实时打字机效果（Claude/OpenAI/Gemini）
//! - **国际化**：支持中英文
//!
//! ## 快速开始
//!
//! ### 作为 CLI 使用
//! ```bash
//! # 安装
//! cargo install gcop-rs
//!
//! # 初始化配置
//! gcop-rs init
//!
//! # 生成 commit message
//! git add .
//! gcop-rs commit
//!
//! # 代码审查
//! gcop-rs review changes
//! ```
//!
//! ### 作为库使用
//! ```ignore
//! use gcop_rs::git::repository::GitRepository;
//! use gcop_rs::git::GitOperations;
//! use gcop_rs::llm::provider::openai::OpenAIProvider;
//! use gcop_rs::llm::LLMProvider;
//! use gcop_rs::config::{ProviderConfig, NetworkConfig};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // 1. 初始化 Git 仓库
//! let repo = GitRepository::open(None)?;
//! let diff = repo.get_staged_diff()?;
//!
//! // 2. 初始化 LLM provider
//! let config = ProviderConfig {
//!     api_key: Some("sk-...".to_string()),
//!     model: "gpt-4o-mini".to_string(),
//!     ..Default::default()
//! };
//! let network_config = NetworkConfig::default();
//! let provider = OpenAIProvider::new(&config, "openai", &network_config, false)?;
//!
//! // 3. 生成 commit message
//! let message = provider.generate_commit_message(&diff, None, None).await?;
//! println!("Generated: {}", message);
//! # Ok(())
//! # }
//! ```
//!
//! ## 核心模块
//! - [`git`] - Git 操作抽象
//! - [`llm`] - LLM provider 接口和实现
//! - [`commands`] - CLI 命令实现
//! - [`config`] - 配置管理
//! - [`error`] - 统一错误类型
//! - [`ui`] - 用户界面工具
//!
//! ## 配置
//! 配置文件位置：
//! - Linux: `~/.config/gcop/config.toml`
//! - macOS: `~/Library/Application Support/gcop/config.toml`
//! - Windows: `%APPDATA%\gcop\config\config.toml`
//! - 项目级（可选）: `<repo>/.gcop/config.toml`
//!
//! 示例配置：
//! ```toml
//! [llm]
//! default_provider = "claude"
//! fallback_providers = ["openai"]
//!
//! [llm.providers.claude]
//! api_key = "sk-ant-..."
//! model = "claude-sonnet-4-5-20250929"
//!
//! [commit]
//! max_retries = 10
//! show_diff_preview = true
//! ```

#[macro_use]
extern crate rust_i18n;

pub mod cli;
pub mod commands;
pub mod config;
pub mod error;
pub mod git;
pub mod llm;
pub mod ui;
pub mod workspace;

// Initialize i18n for library modules
i18n!("locales", fallback = "en");
