//! Provider validation 测试
//!
//! 测试 Claude、OpenAI、Ollama provider 的 validate() 方法

use gcop_rs::config::{NetworkConfig, ProviderConfig};
use gcop_rs::error::{GcopError, Result};
use gcop_rs::llm::LLMProvider;
use gcop_rs::llm::provider::claude::ClaudeProvider;
use gcop_rs::llm::provider::ollama::OllamaProvider;
use gcop_rs::llm::provider::openai::OpenAIProvider;
use mockito::Server;
use std::collections::HashMap;

fn ensure_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn test_network_config() -> NetworkConfig {
    NetworkConfig {
        max_retries: 0, // 禁用重试
        ..Default::default()
    }
}

// ========== Claude Provider Tests ==========

#[tokio::test]
async fn test_claude_validate_success() {
    ensure_crypto_provider();
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"content":[{"type":"text","text":"ok"}]}"#)
        .create_async()
        .await;

    let provider_config = ProviderConfig {
        api_style: None,
        endpoint: Some(server.url()),
        api_key: Some("sk-ant-test-key".to_string()),
        model: "claude-3-haiku-20240307".to_string(),
        max_tokens: None,
        temperature: None,
        extra: HashMap::new(),
    };

    let provider =
        ClaudeProvider::new(&provider_config, "claude", &test_network_config(), false).unwrap();

    assert!(provider.validate().await.is_ok());
    mock.assert_async().await;
}

#[tokio::test]
async fn test_claude_validate_401_unauthorized() {
    ensure_crypto_provider();
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/messages")
        .with_status(401)
        .with_body("Invalid API key")
        .create_async()
        .await;

    let provider_config = ProviderConfig {
        api_style: None,
        endpoint: Some(server.url()),
        api_key: Some("sk-ant-invalid-key".to_string()),
        model: "claude-3-haiku-20240307".to_string(),
        max_tokens: None,
        temperature: None,
        extra: HashMap::new(),
    };

    let provider =
        ClaudeProvider::new(&provider_config, "claude", &test_network_config(), false).unwrap();

    let result: Result<()> = provider.validate().await;
    assert!(result.is_err());

    match result.unwrap_err() {
        GcopError::LlmApi { status, .. } => {
            assert_eq!(status, 401);
        }
        _ => panic!("Expected LlmApi error"),
    }

    mock.assert_async().await;
}

#[tokio::test]
async fn test_claude_validate_429_rate_limit() {
    ensure_crypto_provider();
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/messages")
        .with_status(429)
        .with_body("Rate limit exceeded")
        .create_async()
        .await;

    let provider_config = ProviderConfig {
        api_style: None,
        endpoint: Some(server.url()),
        api_key: Some("sk-ant-test-key".to_string()),
        model: "claude-3-haiku-20240307".to_string(),
        max_tokens: None,
        temperature: None,
        extra: HashMap::new(),
    };

    let provider =
        ClaudeProvider::new(&provider_config, "claude", &test_network_config(), false).unwrap();

    let result: Result<()> = provider.validate().await;
    assert!(result.is_err());

    match result.unwrap_err() {
        GcopError::LlmApi { status, .. } => {
            assert_eq!(status, 429);
        }
        _ => panic!("Expected LlmApi error"),
    }

    mock.assert_async().await;
}

#[tokio::test]
async fn test_claude_validate_empty_api_key() {
    ensure_crypto_provider();
    let provider_config = ProviderConfig {
        api_style: None,
        endpoint: None,
        api_key: Some("".to_string()), // 空 API key
        model: "claude-3-haiku-20240307".to_string(),
        max_tokens: None,
        temperature: None,
        extra: HashMap::new(),
    };

    let provider =
        ClaudeProvider::new(&provider_config, "claude", &test_network_config(), false).unwrap();

    let result: Result<()> = provider.validate().await;
    assert!(result.is_err());

    match result.unwrap_err() {
        GcopError::Config(msg) => {
            assert!(msg.contains("API key is empty"));
        }
        _ => panic!("Expected Config error"),
    }
}

// ========== OpenAI Provider Tests ==========

#[tokio::test]
async fn test_openai_validate_success() {
    ensure_crypto_provider();
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"choices":[{"message":{"content":"ok"},"finish_reason":"stop"}]}"#)
        .create_async()
        .await;

    let provider_config = ProviderConfig {
        api_style: None,
        endpoint: Some(server.url()),
        api_key: Some("sk-test-key".to_string()),
        model: "gpt-4o-mini".to_string(),
        max_tokens: None,
        temperature: None,
        extra: HashMap::new(),
    };

    let provider =
        OpenAIProvider::new(&provider_config, "openai", &test_network_config(), false).unwrap();

    assert!(provider.validate().await.is_ok());
    mock.assert_async().await;
}

#[tokio::test]
async fn test_openai_validate_401_unauthorized() {
    ensure_crypto_provider();
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/chat/completions")
        .with_status(401)
        .with_body(r#"{"error":{"message":"Invalid API key"}}"#)
        .create_async()
        .await;

    let provider_config = ProviderConfig {
        api_style: None,
        endpoint: Some(server.url()),
        api_key: Some("sk-invalid-key".to_string()),
        model: "gpt-4o-mini".to_string(),
        max_tokens: None,
        temperature: None,
        extra: HashMap::new(),
    };

    let provider =
        OpenAIProvider::new(&provider_config, "openai", &test_network_config(), false).unwrap();

    let result: Result<()> = provider.validate().await;
    assert!(result.is_err());

    match result.unwrap_err() {
        GcopError::LlmApi { status, .. } => {
            assert_eq!(status, 401);
        }
        _ => panic!("Expected LlmApi error"),
    }

    mock.assert_async().await;
}

// ========== Ollama Provider Tests ==========

#[tokio::test]
async fn test_ollama_validate_success() {
    ensure_crypto_provider();
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/api/tags")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"models":[{"name":"llama3.2:latest"}]}"#)
        .create_async()
        .await;

    let provider_config = ProviderConfig {
        api_style: None,
        endpoint: Some(format!("{}/api/generate", server.url())),
        api_key: None,
        model: "llama3.2".to_string(),
        max_tokens: None,
        temperature: None,
        extra: HashMap::new(),
    };

    let provider =
        OllamaProvider::new(&provider_config, "ollama", &test_network_config(), false).unwrap();

    assert!(provider.validate().await.is_ok());
    mock.assert_async().await;
}

#[tokio::test]
async fn test_ollama_validate_model_not_found() {
    ensure_crypto_provider();
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/api/tags")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"models":[{"name":"llama3.2:latest"}]}"#)
        .create_async()
        .await;

    let provider_config = ProviderConfig {
        api_style: None,
        endpoint: Some(format!("{}/api/generate", server.url())),
        api_key: None,
        model: "mistral".to_string(), // 不存在的模型
        max_tokens: None,
        temperature: None,
        extra: HashMap::new(),
    };

    let provider =
        OllamaProvider::new(&provider_config, "ollama", &test_network_config(), false).unwrap();

    let result: Result<()> = provider.validate().await;
    assert!(result.is_err());

    match result.unwrap_err() {
        GcopError::Config(msg) => {
            assert!(msg.contains("Model 'mistral' not found"));
            assert!(msg.contains("ollama pull"));
        }
        _ => panic!("Expected Config error"),
    }

    mock.assert_async().await;
}

#[tokio::test]
async fn test_ollama_validate_connection_error() {
    ensure_crypto_provider();
    let provider_config = ProviderConfig {
        api_style: None,
        endpoint: Some("http://localhost:99999/api/generate".to_string()), // 无效端口
        api_key: None,
        model: "llama3.2".to_string(),
        max_tokens: None,
        temperature: None,
        extra: HashMap::new(),
    };

    let provider =
        OllamaProvider::new(&provider_config, "ollama", &test_network_config(), false).unwrap();

    let result: Result<()> = provider.validate().await;
    assert!(result.is_err());

    match result.unwrap_err() {
        GcopError::Network(_) => {
            // 预期的网络错误
        }
        _ => panic!("Expected Network error"),
    }
}
