//! Configuration loading and precedence resolution.
//!
//! Configuration is assembled from user/project files, environment variables,
//! and optional CI overrides.

use config::{Config, Environment, File};
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

use super::structs::{AppConfig, ProviderConfig};
use crate::error::Result;

/// Loads application configuration.
///
/// Effective precedence (high to low):
/// 1. CI overrides (`CI=1` + `GCOP_CI_*`, applied after deserialization)
/// 2. Environment variables (`GCOP__*`, with `__` as nesting separator)
///    - For example: `GCOP__LLM__DEFAULT_PROVIDER=openai`
///    - For example: `GCOP__UI__COLORED=false`
/// 3. Project config (`.gcop/config.toml`, discovered from repo root)
/// 4. User config file (`config.toml` in platform config directory)
/// 5. Rust defaults (`Default` + `serde(default)`)
///
/// Sources are added from low to high priority (`user -> project -> env`)
/// because later `config-rs` sources override earlier ones.
/// CI overrides are applied last.
pub fn load_config() -> Result<AppConfig> {
    load_config_from_path(get_config_path(), find_project_config())
}

/// Loads configuration from explicit paths (test-friendly entrypoint).
///
/// Passing `None` skips the corresponding file source.
pub(crate) fn load_config_from_path(
    config_path: Option<PathBuf>,
    project_config_path: Option<PathBuf>,
) -> Result<AppConfig> {
    let mut builder = Config::builder();

    // User config (lowest priority source).
    if let Some(config_path) = config_path
        && config_path.exists()
    {
        builder = builder.add_source(File::from(config_path));
    }

    // Project config (overrides user config).
    if let Some(ref project_path) = project_config_path
        && project_path.exists()
    {
        // Security check: project config should not include `api_key`.
        check_project_config_security(project_path);
        builder = builder.add_source(File::from(project_path.clone()));
    }

    // Environment variables (highest source priority in config-rs builder order).
    // Double underscore is used as nesting separator:
    // `GCOP__LLM__DEFAULT_PROVIDER` -> `llm.default_provider`.
    builder = builder.add_source(
        Environment::with_prefix("GCOP")
            .separator("__")
            .try_parsing(true),
    );

    // Build and deserialize merged sources.
    let config = builder.build()?;
    let mut app_config: AppConfig = config.try_deserialize()?;

    // CI mode overrides (highest effective priority).
    apply_ci_mode_overrides(&mut app_config)?;

    // Validate final config.
    app_config.validate()?;

    Ok(app_config)
}

/// Finds project-level `.gcop/config.toml`.
///
/// Resolves the repository root via [`crate::git::find_git_root`], then checks
/// for `.gcop/config.toml` at that root.
/// `init --project` always creates `.gcop/` at the repository root, so no
/// upward traversal is needed once the root is known.
pub(crate) fn find_project_config() -> Option<PathBuf> {
    let root = crate::git::find_git_root()?;
    let candidate = root.join(".gcop").join("config.toml");
    candidate.exists().then_some(candidate)
}

/// Warns when project-level config contains secrets.
///
/// If project config contains an `api_key`, prints warnings encouraging users to
/// move secrets into user-level config or environment variables.
fn check_project_config_security(path: &Path) {
    if let Ok(content) = std::fs::read_to_string(path) {
        // Detect `api_key` in non-comment lines.
        let has_api_key = content.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with('#') && trimmed.contains("api_key")
        });
        if has_api_key {
            eprintln!("{}", rust_i18n::t!("config.project_api_key_warning_line1"));
            eprintln!("{}", rust_i18n::t!("config.project_api_key_warning_line2"));
            eprintln!("{}", rust_i18n::t!("config.project_api_key_warning_line3"));
        }
    }
}

/// Applies CI-mode environment overrides.
///
/// When `CI=1`, provider config is built from:
/// - `GCOP_CI_PROVIDER`: "claude", "openai", "ollama" or "gemini" (required)
/// - `GCOP_CI_API_KEY`: API key (required)
/// - `GCOP_CI_MODEL`: model name (optional, has a provider-specific default)
/// - `GCOP_CI_ENDPOINT`: custom endpoint (optional)
///
/// The resulting provider is inserted as `"ci"` and set as `default_provider`.
fn apply_ci_mode_overrides(config: &mut AppConfig) -> Result<()> {
    use std::env;

    // Check whether CI mode is enabled.
    let ci_enabled = env::var("CI").ok().as_deref() == Some("1");

    if !ci_enabled {
        return Ok(());
    }

    // Read GCOP_CI_PROVIDER (required).
    let provider_type = env::var("GCOP_CI_PROVIDER").map_err(|_| {
        crate::error::GcopError::Config(rust_i18n::t!("config.ci_provider_not_set").to_string())
    })?;

    // Validate provider type.
    let api_style: super::structs::ApiStyle = provider_type.parse().map_err(|_| {
        crate::error::GcopError::Config(
            rust_i18n::t!(
                "config.ci_provider_invalid",
                provider = provider_type.as_str()
            )
            .to_string(),
        )
    })?;

    // Read GCOP_CI_API_KEY (required).
    let api_key = env::var("GCOP_CI_API_KEY").map_err(|_| {
        crate::error::GcopError::Config(rust_i18n::t!("config.ci_api_key_not_set").to_string())
    })?;

    // Read GCOP_CI_MODEL (optional, with default).
    let model = env::var("GCOP_CI_MODEL").unwrap_or_else(|_| api_style.default_model().to_string());

    // Read GCOP_CI_ENDPOINT (optional).
    let endpoint = env::var("GCOP_CI_ENDPOINT").ok();

    // Build provider config.
    let provider_config = ProviderConfig {
        api_style: Some(api_style),
        endpoint,
        api_key: Some(api_key),
        model,
        max_tokens: None,
        temperature: None,
        extra: Default::default(),
    };

    // Inject into runtime config.
    config
        .llm
        .providers
        .insert("ci".to_string(), provider_config);
    config.llm.default_provider = "ci".to_string();

    tracing::info!("CI mode enabled, using GCOP_CI_PROVIDER={}", api_style);

    Ok(())
}

/// Returns platform-specific config file path.
///
/// Path format: `<config_dir>/config.toml`.
fn get_config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "gcop").map(|dirs| dirs.config_dir().join("config.toml"))
}

/// Returns platform-specific config directory path.
///
/// Used by commands that need direct directory access (for example, init and validate flows).
pub fn get_config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "gcop").map(|dirs| dirs.config_dir().to_path_buf())
}
