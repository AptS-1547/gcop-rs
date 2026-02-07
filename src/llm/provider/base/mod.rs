//! Provider 公共抽象和辅助函数
//!
//! 提取各 Provider 的通用逻辑，减少重复代码。
//!
//! 模块结构：
//! - `config` - 配置提取工具函数
//! - `response` - 响应处理和 JSON 清理
//! - `retry` - HTTP 请求发送与重试逻辑

pub mod config;
pub mod response;
pub mod retry;

// 重新导出常用函数，保持向后兼容
pub use config::*;
pub use response::*;
pub use retry::send_llm_request;
