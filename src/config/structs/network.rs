//! Network and HTTP configuration structures.

use serde::{Deserialize, Serialize};

use crate::error::{GcopError, Result};

/// Network configuration.
///
/// Controls timeout and retry behavior for HTTP requests.
///
/// # Fields
/// - `request_timeout`: HTTP request timeout in seconds (default: `120`)
/// - `connect_timeout`: HTTP connect timeout in seconds (default: `10`)
/// - `max_retries`: max retries for LLM API requests (default: `3`)
/// - `retry_delay_ms`: initial retry delay in milliseconds (default: `1000`)
/// - `max_retry_delay_ms`: max retry delay in milliseconds (default: `60000`)
///
/// # Example
/// ```toml
/// [network]
/// request_timeout = 30
/// connect_timeout = 10
/// max_retries = 3
/// retry_delay_ms = 1000
/// max_retry_delay_ms = 60000
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkConfig {
    /// HTTP request timeout in seconds.
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,

    /// HTTP connect timeout in seconds.
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,

    /// Maximum retries for LLM API requests.
    #[serde(default = "default_network_max_retries")]
    pub max_retries: usize,

    /// Initial retry delay in milliseconds.
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,

    /// Maximum retry delay in milliseconds.
    #[serde(default = "default_max_retry_delay_ms")]
    pub max_retry_delay_ms: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            request_timeout: default_request_timeout(),
            connect_timeout: default_connect_timeout(),
            max_retries: default_network_max_retries(),
            retry_delay_ms: default_retry_delay_ms(),
            max_retry_delay_ms: default_max_retry_delay_ms(),
        }
    }
}

impl NetworkConfig {
    /// Validates network configuration.
    pub fn validate(&self) -> Result<()> {
        if self.request_timeout == 0 {
            return Err(GcopError::Config(
                "network.request_timeout cannot be 0".into(),
            ));
        }
        if self.connect_timeout == 0 {
            return Err(GcopError::Config(
                "network.connect_timeout cannot be 0".into(),
            ));
        }
        Ok(())
    }
}

fn default_request_timeout() -> u64 {
    120
}

fn default_connect_timeout() -> u64 {
    10
}

fn default_network_max_retries() -> usize {
    3
}

fn default_retry_delay_ms() -> u64 {
    1000
}

fn default_max_retry_delay_ms() -> u64 {
    60_000 // 60 seconds
}
