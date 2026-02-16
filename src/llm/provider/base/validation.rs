//! Provider validation helper function
//!
//! Provide common API verification logic to reduce duplicate code in provider implementation.

use reqwest::Client;
use serde::Serialize;

use crate::error::{GcopError, Result};

/// Verify API key is empty
///
/// # Parameters
/// - `api_key` - API key to verify
///
/// # Returns
/// - If API key is empty, return `GcopError::Config` error
/// - Otherwise return `Ok(())`
///
/// # Example
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

/// Send a test request to verify the API endpoint
///
/// Send a minimal test request to the LLM provider API to verify:
/// - Is the network connection normal?
/// - Is the API key valid?
/// - Is the endpoint configuration correct?
///
/// #Type parameters
/// - `T` - request body type (must implement `Serialize`)
///
/// # Parameters
/// - `client` - HTTP client
/// - `endpoint` - API endpoint URL
/// - `headers` - HTTP headers (such as Authorization, x-api-key, etc.)
/// - `test_request` - Test request body (usually set `max_tokens=1` to minimize API cost)
/// - `provider_name` - Provider name (used for log and error messages)
///
/// # Returns
/// - If verification is successful, return `Ok(())`
/// - If the request fails, return `GcopError::Network` error
/// - If the API returns an error status code, return the `GcopError::LlmApi` error
///
/// # Example
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

    // Build request
    let mut request_builder = client
        .post(endpoint)
        .header("Content-Type", "application/json");

    // Add custom headers
    for (key, value) in headers {
        request_builder = request_builder.header(*key, *value);
    }

    // Send request
    let response = request_builder
        .json(test_request)
        .send()
        .await
        .map_err(GcopError::Network)?;

    // Check status code
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
