// Configure module testing
//
// This file contains all configuration related tests.

use super::*;
use pretty_assertions::assert_eq;
use serial_test::serial;
use std::env;

/// RAII environment variable guard to ensure cleanup after testing
struct EnvGuard {
    key: String,
    original: Option<String>,
}

impl EnvGuard {
    fn set(key: &str, value: &str) -> Self {
        let original = env::var(key).ok();
        // SAFETY: It is safe to modify environment variables in the test environment, and use serial_test to ensure serial execution
        unsafe { env::set_var(key, value) };
        Self {
            key: key.to_string(),
            original,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: It is safe to modify environment variables in the test environment
        match &self.original {
            Some(v) => unsafe { env::set_var(&self.key, v) },
            None => unsafe { env::remove_var(&self.key) },
        }
    }
}

// === Default value testing (testing the Default implementation of structs.rs) ===

#[test]
fn test_app_config_default_llm() {
    let config = AppConfig::default();
    assert_eq!(config.llm.default_provider, "claude");
}

#[test]
fn test_app_config_default_commit() {
    let config = AppConfig::default();
    assert!(config.commit.show_diff_preview);
    assert!(config.commit.allow_edit);
    assert_eq!(config.commit.max_retries, 10);
}

#[test]
fn test_app_config_default_network() {
    let config = AppConfig::default();
    assert_eq!(config.network.request_timeout, 120);
    assert_eq!(config.network.connect_timeout, 10);
    assert_eq!(config.network.max_retries, 3);
    assert_eq!(config.network.retry_delay_ms, 1000);
    assert_eq!(config.network.max_retry_delay_ms, 60_000);
}

#[test]
fn test_app_config_default_ui() {
    let config = AppConfig::default();
    assert!(config.ui.colored);
    assert!(config.ui.streaming);
}

#[test]
fn test_app_config_default_review() {
    let config = AppConfig::default();
    assert_eq!(config.review.min_severity, "info");
}

#[test]
fn test_app_config_default_file() {
    let config = AppConfig::default();
    assert_eq!(config.file.max_size, 10 * 1024 * 1024);
}

// === Configuration loading test ===

#[test]
#[serial]
fn test_load_config_succeeds() {
    // Verify that load_config does not crash (without reading user configuration files)
    let result = loader::load_config_from_path(None, None);
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_load_config_returns_valid_config() {
    let config = loader::load_config_from_path(None, None).unwrap();
    // Verify that the configuration has reasonable values
    assert!(!config.llm.default_provider.is_empty());
    assert!(config.commit.max_retries > 0);
    assert!(config.network.request_timeout > 0);
}

// === Path function test ===

#[test]
fn test_get_config_dir_returns_valid_path() {
    let config_dir = loader::get_config_dir();
    assert!(config_dir.is_some());
    let path = config_dir.unwrap();
    // The path should contain "gcop"
    assert!(path.to_string_lossy().contains("gcop"));
}

#[test]
fn test_get_config_path_has_toml_suffix() {
    let config_dir = loader::get_config_dir();
    assert!(config_dir.is_some());
    // config.toml should be in the configuration directory
    let config_path = config_dir.unwrap().join("config.toml");
    assert!(config_path.to_string_lossy().ends_with("config.toml"));
}

// === Environment variable coverage test ===

#[test]
#[serial]
fn test_env_guard_sets_and_restores() {
    let key = "GCOP_TEST_VAR";

    // Make sure it doesn't exist before testing
    // SAFETY: test environment
    unsafe { env::remove_var(key) };

    {
        let _guard = EnvGuard::set(key, "test_value");
        assert_eq!(env::var(key).unwrap(), "test_value");
    }

    // guard should be restored (deleted) after release
    assert!(env::var(key).is_err());
}

#[test]
#[serial]
fn test_env_var_can_be_read() {
    let _guard = EnvGuard::set("GCOP__UI__COLORED", "false");
    // Verify environment variables are set correctly
    assert_eq!(env::var("GCOP__UI__COLORED").unwrap(), "false");
}

#[test]
#[serial]
fn test_env_var_llm_default_provider() {
    // Verify whether the GCOP__LLM__DEFAULT_PROVIDER environment variable is effective
    // Note: Use double underscores to indicate nesting levels
    let _guard = EnvGuard::set("GCOP__LLM__DEFAULT_PROVIDER", "test_provider");
    let config = loader::load_config_from_path(None, None).unwrap();
    // Environment variables have the highest priority and should override configuration files.
    assert_eq!(config.llm.default_provider, "test_provider");
}

// === CI mode testing ===

#[test]
#[serial]
fn test_ci_mode_enabled_with_ci_env() {
    let _ci = EnvGuard::set("CI", "1");
    let _type = EnvGuard::set("GCOP_CI_PROVIDER", "claude");
    let _key = EnvGuard::set("GCOP_CI_API_KEY", "sk-test");

    let config = loader::load_config_from_path(None, None).unwrap();

    // CI mode should set default_provider to "ci"
    assert_eq!(config.llm.default_provider, "ci");

    // There should be a provider named "ci"
    assert!(config.llm.providers.contains_key("ci"));

    let ci_provider = &config.llm.providers["ci"];
    assert_eq!(ci_provider.api_style, Some(structs::ApiStyle::Claude));
    assert_eq!(ci_provider.api_key, Some("sk-test".to_string()));
    assert_eq!(ci_provider.model, "claude-sonnet-4-5-20250929"); // default value
}

#[test]
#[serial]
fn test_ci_mode_with_custom_model() {
    let _ci = EnvGuard::set("CI", "1");
    let _type = EnvGuard::set("GCOP_CI_PROVIDER", "ollama");
    let _key = EnvGuard::set("GCOP_CI_API_KEY", "dummy");
    let _model = EnvGuard::set("GCOP_CI_MODEL", "llama3.1");

    let config = loader::load_config_from_path(None, None).unwrap();

    let ci_provider = &config.llm.providers["ci"];
    assert_eq!(ci_provider.api_style, Some(structs::ApiStyle::Ollama));
    assert_eq!(ci_provider.model, "llama3.1"); // custom value
}

#[test]
#[serial]
fn test_ci_mode_with_custom_endpoint() {
    let _ci = EnvGuard::set("CI", "1");
    let _type = EnvGuard::set("GCOP_CI_PROVIDER", "claude");
    let _key = EnvGuard::set("GCOP_CI_API_KEY", "sk-test");
    let _endpoint = EnvGuard::set("GCOP_CI_ENDPOINT", "https://custom-api.com");

    let config = loader::load_config_from_path(None, None).unwrap();

    let ci_provider = &config.llm.providers["ci"];
    assert_eq!(
        ci_provider.endpoint,
        Some("https://custom-api.com".to_string())
    );
}

#[test]
#[serial]
fn test_ci_mode_missing_provider_type() {
    let _ci = EnvGuard::set("CI", "1");
    let _key = EnvGuard::set("GCOP_CI_API_KEY", "sk-test");
    // GCOP_CI_PROVIDER not set

    let result = loader::load_config_from_path(None, None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("GCOP_CI_PROVIDER not set")
    );
}

#[test]
#[serial]
fn test_ci_mode_missing_api_key() {
    let _ci = EnvGuard::set("CI", "1");
    let _type = EnvGuard::set("GCOP_CI_PROVIDER", "claude");
    // GCOP_CI_API_KEY not set

    let result = loader::load_config_from_path(None, None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("GCOP_CI_API_KEY not set")
    );
}

#[test]
#[serial]
fn test_ci_mode_invalid_provider_type() {
    let _ci = EnvGuard::set("CI", "1");
    let _type = EnvGuard::set("GCOP_CI_PROVIDER", "invalid");
    let _key = EnvGuard::set("GCOP_CI_API_KEY", "sk-test");

    let result = loader::load_config_from_path(None, None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid GCOP_CI_PROVIDER")
    );
}

#[test]
#[serial]
fn test_ci_mode_disabled_by_default() {
    // Without setting CI=1, the "ci" provider should not be created
    let config = loader::load_config_from_path(None, None).unwrap();
    assert!(!config.llm.providers.contains_key("ci"));
    assert_eq!(config.llm.default_provider, "claude"); // default value
}

// === validate: default_provider / fallback_providers existence check ===

#[test]
fn test_validate_default_provider_not_in_providers() {
    let mut config = AppConfig::default();
    config.llm.default_provider = "nonexistent".to_string();
    config
        .llm
        .providers
        .insert("claude".to_string(), make_test_provider());

    let result = config.validate();
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("nonexistent"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_validate_default_provider_ok_when_providers_empty() {
    // Default configuration: default_provider = "claude", providers = {}
    // No error will be reported when providers is empty (the user has not configured it yet, so processing is delayed until runtime)
    let config = AppConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_validate_default_provider_exists() {
    let mut config = AppConfig::default();
    config.llm.default_provider = "claude".to_string();
    config
        .llm
        .providers
        .insert("claude".to_string(), make_test_provider());

    assert!(config.validate().is_ok());
}

#[test]
fn test_validate_fallback_provider_not_in_providers() {
    let mut config = AppConfig::default();
    config.llm.default_provider = "claude".to_string();
    config
        .llm
        .providers
        .insert("claude".to_string(), make_test_provider());
    config.llm.fallback_providers = vec!["typo_openai".to_string()];

    let result = config.validate();
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("typo_openai"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_validate_fallback_providers_all_exist() {
    let mut config = AppConfig::default();
    config.llm.default_provider = "claude".to_string();
    config
        .llm
        .providers
        .insert("claude".to_string(), make_test_provider());
    config
        .llm
        .providers
        .insert("openai".to_string(), make_test_provider());
    config.llm.fallback_providers = vec!["openai".to_string()];

    assert!(config.validate().is_ok());
}

#[test]
fn test_validate_fallback_providers_empty_is_ok() {
    let mut config = AppConfig::default();
    config.llm.default_provider = "claude".to_string();
    config
        .llm
        .providers
        .insert("claude".to_string(), make_test_provider());
    config.llm.fallback_providers = vec![];

    assert!(config.validate().is_ok());
}

/// Construct a minimally legal ProviderConfig for testing
fn make_test_provider() -> structs::ProviderConfig {
    structs::ProviderConfig {
        api_style: None,
        endpoint: None,
        api_key: Some("sk-test-key".to_string()),
        model: "test-model".to_string(),
        max_tokens: None,
        temperature: None,
        extra: Default::default(),
    }
}

// === Default value consistency test ===

#[test]
fn test_serde_empty_config_matches_default() {
    // Deserialize through the empty builder of the config crate and verify that it is consistent with AppConfig::default()
    // This is the real path of load_config(): when there is no configuration file or environment variables, go to config crate -> serde(default)
    let config = config::Config::builder().build().unwrap();
    let deserialized: AppConfig = config.try_deserialize().unwrap();
    let default_config = AppConfig::default();

    // LLM
    assert_eq!(
        deserialized.llm.default_provider,
        default_config.llm.default_provider
    );
    assert_eq!(
        deserialized.llm.max_diff_size,
        default_config.llm.max_diff_size
    );

    // Commit
    assert_eq!(
        deserialized.commit.show_diff_preview,
        default_config.commit.show_diff_preview
    );
    assert_eq!(
        deserialized.commit.allow_edit,
        default_config.commit.allow_edit
    );
    assert_eq!(
        deserialized.commit.max_retries,
        default_config.commit.max_retries
    );

    // Review
    assert_eq!(
        deserialized.review.min_severity,
        default_config.review.min_severity
    );

    // UI
    assert_eq!(deserialized.ui.colored, default_config.ui.colored);
    assert_eq!(deserialized.ui.streaming, default_config.ui.streaming);

    // Network
    assert_eq!(
        deserialized.network.request_timeout,
        default_config.network.request_timeout
    );
    assert_eq!(
        deserialized.network.connect_timeout,
        default_config.network.connect_timeout
    );
    assert_eq!(
        deserialized.network.max_retries,
        default_config.network.max_retries
    );
    assert_eq!(
        deserialized.network.retry_delay_ms,
        default_config.network.retry_delay_ms
    );
    assert_eq!(
        deserialized.network.max_retry_delay_ms,
        default_config.network.max_retry_delay_ms
    );

    // File
    assert_eq!(deserialized.file.max_size, default_config.file.max_size);

    // Commit convention
    assert_eq!(
        deserialized.commit.convention,
        default_config.commit.convention
    );
}

// === CommitConvention Test ===

#[test]
fn test_commit_convention_default() {
    let conv = structs::CommitConvention::default();
    assert_eq!(conv.style, structs::ConventionStyle::Conventional);
    assert!(conv.types.is_none());
    assert!(conv.template.is_none());
    assert!(conv.extra_prompt.is_none());
}

#[test]
fn test_commit_config_default_convention_is_none() {
    let config = AppConfig::default();
    assert!(config.commit.convention.is_none());
}

#[test]
fn test_convention_style_serde_roundtrip() {
    // Verify serialization/deserialization of ConventionStyle
    let styles = vec![
        (structs::ConventionStyle::Conventional, "\"conventional\""),
        (structs::ConventionStyle::Gitmoji, "\"gitmoji\""),
        (structs::ConventionStyle::Custom, "\"custom\""),
    ];
    for (style, expected_json) in styles {
        let json = serde_json::to_string(&style).unwrap();
        assert_eq!(json, expected_json);
        let deserialized: structs::ConventionStyle = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, style);
    }
}

// === Project configuration three-tier priority testing ===

#[test]
#[serial]
fn test_project_config_overrides_user_config() {
    use std::io::Write;

    let user_dir = tempfile::tempdir().unwrap();
    let project_dir = tempfile::tempdir().unwrap();

    // User configuration: default_provider = "claude"
    let user_config = user_dir.path().join("config.toml");
    let mut f = std::fs::File::create(&user_config).unwrap();
    writeln!(f, "[llm]\ndefault_provider = \"claude\"").unwrap();

    // Project configuration: default_provider = "openai"
    let project_config = project_dir.path().join("config.toml");
    let mut f = std::fs::File::create(&project_config).unwrap();
    writeln!(f, "[llm]\ndefault_provider = \"openai\"").unwrap();

    let config = loader::load_config_from_path(Some(user_config), Some(project_config)).unwrap();

    // Project configuration should override user configuration
    assert_eq!(config.llm.default_provider, "openai");
}

#[test]
#[serial]
fn test_env_overrides_project_config() {
    use std::io::Write;

    let project_dir = tempfile::tempdir().unwrap();

    // Project configuration: default_provider = "openai"
    let project_config = project_dir.path().join("config.toml");
    let mut f = std::fs::File::create(&project_config).unwrap();
    writeln!(f, "[llm]\ndefault_provider = \"openai\"").unwrap();

    // Environment variable override
    let _guard = EnvGuard::set("GCOP__LLM__DEFAULT_PROVIDER", "gemini");

    let config = loader::load_config_from_path(None, Some(project_config)).unwrap();

    // Environment variables should override project configuration
    assert_eq!(config.llm.default_provider, "gemini");
}

#[test]
#[serial]
fn test_load_config_with_no_project_config() {
    // Should work fine without project configuration
    let config = loader::load_config_from_path(None, None).unwrap();
    assert_eq!(config.llm.default_provider, "claude"); // default value
}

// === CommitConvention TOML parsing test ===

#[test]
fn test_convention_from_toml() {
    use config::{Config, File, FileFormat};

    let toml_content = r#"
[commit.convention]
style = "gitmoji"
types = ["feat", "fix", "docs"]
template = "{type}: {subject}"
extra_prompt = "Use English only"
"#;

    let config = Config::builder()
        .add_source(File::from_str(toml_content, FileFormat::Toml))
        .build()
        .unwrap();
    let app_config: AppConfig = config.try_deserialize().unwrap();

    let conv = app_config.commit.convention.unwrap();
    assert_eq!(conv.style, structs::ConventionStyle::Gitmoji);
    assert_eq!(
        conv.types,
        Some(vec![
            "feat".to_string(),
            "fix".to_string(),
            "docs".to_string()
        ])
    );
    assert_eq!(conv.template, Some("{type}: {subject}".to_string()));
    assert_eq!(conv.extra_prompt, Some("Use English only".to_string()));
}

#[test]
fn test_convention_partial_from_toml() {
    use config::{Config, File, FileFormat};

    let toml_content = r#"
[commit.convention]
style = "conventional"
"#;

    let config = Config::builder()
        .add_source(File::from_str(toml_content, FileFormat::Toml))
        .build()
        .unwrap();
    let app_config: AppConfig = config.try_deserialize().unwrap();

    let conv = app_config.commit.convention.unwrap();
    assert_eq!(conv.style, structs::ConventionStyle::Conventional);
    assert!(conv.types.is_none());
    assert!(conv.template.is_none());
    assert!(conv.extra_prompt.is_none());
}
