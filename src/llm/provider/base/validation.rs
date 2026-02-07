//! Provider 验证辅助函数
//!
//! 提供通用的 API 验证逻辑，减少 provider 实现中的重复代码。

use reqwest::Client;
use serde::Serialize;

use crate::error::{GcopError, Result};

/// 验证 API key 是否为空
///
/// # 参数
/// - `api_key` - 要验证的 API key
///
/// # 返回
/// - 如果 API key 为空，返回 `GcopError::Config` 错误
/// - 否则返回 `Ok(())`
///
/// # 示例
/// ```
/// use gcop_rs::llm::provider::base::validation::validate_api_key;
///
/// assert!(validate_api_key("sk-test").is_ok());
/// assert!(validate_api_key("").is_err());
/// ```
pub fn validate_api_key(api_key: &str) -> Result<()> {
    if api_key.is_empty() {
        return Err(GcopError::Config(
            rust_i18n::t!("provider.api_key_empty").to_string(),
        ));
    }
    Ok(())
}

/// 发送测试请求以验证 API 端点
///
/// 向 LLM provider API 发送一个最小的测试请求，验证：
/// - 网络连接是否正常
/// - API key 是否有效
/// - 端点配置是否正确
///
/// # 类型参数
/// - `T` - 请求体类型（必须实现 `Serialize`）
///
/// # 参数
/// - `client` - HTTP 客户端
/// - `endpoint` - API 端点 URL
/// - `headers` - HTTP headers（如 Authorization, x-api-key 等）
/// - `test_request` - 测试请求体（通常设置 `max_tokens=1` 以最小化 API 成本）
/// - `provider_name` - Provider 名称（用于日志和错误消息）
///
/// # 返回
/// - 如果验证成功，返回 `Ok(())`
/// - 如果请求失败，返回 `GcopError::Network` 错误
/// - 如果 API 返回错误状态码，返回 `GcopError::LlmApi` 错误
///
/// # 示例
/// ```ignore
/// use gcop_rs::llm::provider::base::validation::validate_http_endpoint;
/// use reqwest::Client;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct TestRequest {
///     model: String,
///     max_tokens: u32,
/// }
///
/// # async fn example() -> anyhow::Result<()> {
/// let client = Client::new();
/// let request = TestRequest {
///     model: "test-model".to_string(),
///     max_tokens: 1,
/// };
///
/// validate_http_endpoint(
///     &client,
///     "https://api.example.com/v1/chat",
///     &[("Authorization", "Bearer sk-test")],
///     &request,
///     "TestProvider",
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn validate_http_endpoint<T: Serialize>(
    client: &Client,
    endpoint: &str,
    headers: &[(&str, &str)],
    test_request: &T,
    provider_name: &str,
) -> Result<()> {
    tracing::debug!("Validating {} API connection...", provider_name);

    // 构建请求
    let mut request_builder = client
        .post(endpoint)
        .header("Content-Type", "application/json");

    // 添加自定义 headers
    for (key, value) in headers {
        request_builder = request_builder.header(*key, *value);
    }

    // 发送请求
    let response = request_builder
        .json(test_request)
        .send()
        .await
        .map_err(GcopError::Network)?;

    // 检查状态码
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(GcopError::LlmApi {
            status: status.as_u16(),
            message: rust_i18n::t!(
                "provider.api_validation_failed",
                provider = provider_name,
                body = body
            )
            .to_string(),
        });
    }

    tracing::debug!("{} API connection validated successfully", provider_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_api_key_success() {
        assert!(validate_api_key("sk-test-key").is_ok());
        assert!(validate_api_key("a").is_ok());
    }

    #[test]
    fn test_validate_api_key_empty() {
        let result = validate_api_key("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GcopError::Config(_)));
    }

    #[tokio::test]
    async fn test_validate_http_endpoint_success() {
        use crate::llm::provider::test_utils::ensure_crypto_provider;
        use mockito::Server;
        use serde::Serialize;
        ensure_crypto_provider();

        #[derive(Serialize)]
        struct TestRequest {
            test: String,
        }

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ok":true}"#)
            .create_async()
            .await;

        let client = Client::new();
        let request = TestRequest {
            test: "test".to_string(),
        };

        let result = validate_http_endpoint(
            &client,
            &format!("{}/test", server.url()),
            &[("Authorization", "Bearer test")],
            &request,
            "TestProvider",
        )
        .await;

        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_validate_http_endpoint_auth_error() {
        use crate::llm::provider::test_utils::ensure_crypto_provider;
        use mockito::Server;
        use serde::Serialize;
        ensure_crypto_provider();

        #[derive(Serialize)]
        struct TestRequest {
            test: String,
        }

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let client = Client::new();
        let request = TestRequest {
            test: "test".to_string(),
        };

        let result = validate_http_endpoint(
            &client,
            &format!("{}/test", server.url()),
            &[("Authorization", "Bearer invalid")],
            &request,
            "TestProvider",
        )
        .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GcopError::LlmApi { status: 401, .. }
        ));
        mock.assert_async().await;
    }
}
