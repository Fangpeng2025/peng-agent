//! 网页领域工具处理器
//!
//! 提供网络请求相关工具：
//! - `web_search` — 网络搜索（需配置搜索 API 端点）
//! - `web_fetch` — 获取网页内容（GET 请求）
//! - `web_post` — 发送 POST 请求
//!
//! 使用 reqwest HTTP 客户端执行网络操作。

use std::time::Duration;

use crate::config::KernelConfig;
use crate::llm::ToolDefinition;
use crate::types::{Error, Result, ToolDomain};

use super::router::ToolHandler;

// ============================================================================
// WebHandler
// ============================================================================

/// 网页领域工具处理器
///
/// 通过 reqwest 执行 HTTP 请求。搜索功能需要额外配置搜索 API 端点。
pub struct WebHandler {
    /// HTTP 客户端
    client: reqwest::Client,
    /// 搜索 API 端点（可选）
    search_api_endpoint: String,
    /// 搜索 API 密钥（可选）
    search_api_key: String,
}

impl WebHandler {
    /// 创建网页领域处理器
    pub fn new(_config: &KernelConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            search_api_endpoint: String::new(),
            search_api_key: String::new(),
        }
    }

    /// 网络搜索
    ///
    /// 如果配置了搜索 API 端点，调用该端点执行搜索；
    /// 否则返回未配置提示。
    fn web_search(&self, params: &str) -> Result<String> {
        let p: SearchParams = parse_params(params)?;

        if self.search_api_endpoint.is_empty() {
            return Ok(serde_json::json!({
                "status": "not_configured",
                "message": "Web search not configured. Set search_api_endpoint in config.",
                "query": p.query
            })
            .to_string());
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Tool(format!("创建 tokio runtime 失败: {e}")))?;

        let client = self.client.clone();
        let endpoint = self.search_api_endpoint.clone();
        let api_key = self.search_api_key.clone();
        let query = p.query;

        rt.block_on(async move {
            let mut request = client
                .get(&endpoint)
                .query(&[("q", &query)]);

            if !api_key.is_empty() {
                request = request.header("Authorization", format!("Bearer {api_key}"));
            }

            let response = request.send().await.map_err(|e| {
                Error::Tool(format!("搜索请求失败: {e}"))
            })?;

            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(Error::Tool(format!(
                    "搜索 API 返回错误 (HTTP {}): {body}",
                    status.as_u16()
                )));
            }

            let body = response.text().await.map_err(|e| {
                Error::Tool(format!("读取搜索响应失败: {e}"))
            })?;

            Ok(body)
        })
    }

    /// 获取网页内容
    ///
    /// 发送 GET 请求并返回响应体文本（截断至 10000 字符）。
    fn web_fetch(&self, params: &str) -> Result<String> {
        let p: FetchParams = parse_params(params)?;

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Tool(format!("创建 tokio runtime 失败: {e}")))?;

        let client = self.client.clone();
        let url = p.url;
        let max_chars = p.max_chars.unwrap_or(10_000);

        rt.block_on(async move {
            let response = client.get(&url).send().await.map_err(|e| {
                Error::Tool(format!("请求失败 ({url}): {e}"))
            })?;

            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(Error::Tool(format!(
                    "HTTP 请求返回错误 (HTTP {}): {body}",
                    status.as_u16()
                )));
            }

            let body = response.text().await.map_err(|e| {
                Error::Tool(format!("读取响应体失败: {e}"))
            })?;

            // 截断过长的响应
            if body.len() > max_chars {
                Ok(format!(
                    "{}\n\n[... 内容已截断，原始长度: {} 字符 ...]",
                    &body[..max_chars],
                    body.len()
                ))
            } else {
                Ok(body)
            }
        })
    }

    /// 发送 POST 请求
    fn web_post(&self, params: &str) -> Result<String> {
        let p: PostParams = parse_params(params)?;

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Tool(format!("创建 tokio runtime 失败: {e}")))?;

        let client = self.client.clone();
        let url = p.url;
        let body = p.body;
        let content_type = p.content_type.unwrap_or_else(|| "application/json".to_string());

        rt.block_on(async move {
            let response = client
                .post(&url)
                .header("Content-Type", &content_type)
                .body(body)
                .send()
                .await
                .map_err(|e| Error::Tool(format!("POST 请求失败 ({url}): {e}")))?;

            let status = response.status();
            let response_body = response.text().await.map_err(|e| {
                Error::Tool(format!("读取响应体失败: {e}"))
            })?;

            if !status.is_success() {
                return Err(Error::Tool(format!(
                    "POST 请求返回错误 (HTTP {}): {response_body}",
                    status.as_u16()
                )));
            }

            // 截断过长的响应
            if response_body.len() > 10_000 {
                Ok(format!(
                    "{}\n\n[... 内容已截断，原始长度: {} 字符 ...]",
                    &response_body[..10_000],
                    response_body.len()
                ))
            } else {
                Ok(response_body)
            }
        })
    }
}

impl ToolHandler for WebHandler {
    fn domain(&self) -> ToolDomain {
        ToolDomain::Web
    }

    fn execute(&self, tool_name: &str, params: &str) -> Result<String> {
        match tool_name {
            "web_search" => self.web_search(params),
            "web_fetch" => self.web_fetch(params),
            "web_post" => self.web_post(params),
            _ => Err(Error::Tool(format!(
                "网页领域未知工具: {tool_name}"
            ))),
        }
    }

    fn tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition::function(
                "web_search",
                "网络搜索。通过配置的搜索 API 端点执行搜索查询。如未配置搜索端点，将返回提示信息。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索查询关键词"
                        }
                    },
                    "required": ["query"]
                }),
            ),
            ToolDefinition::function(
                "web_fetch",
                "获取网页内容。发送 HTTP GET 请求并返回响应体文本，默认截断至 10000 字符。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "要获取的网页 URL"
                        },
                        "max_chars": {
                            "type": "integer",
                            "description": "最大返回字符数（默认 10000）",
                            "default": 10000
                        }
                    },
                    "required": ["url"]
                }),
            ),
            ToolDefinition::function(
                "web_post",
                "发送 HTTP POST 请求。支持自定义请求体和 Content-Type。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "请求目标 URL"
                        },
                        "body": {
                            "type": "string",
                            "description": "请求体内容"
                        },
                        "content_type": {
                            "type": "string",
                            "description": "Content-Type 头（默认 application/json）",
                            "default": "application/json"
                        }
                    },
                    "required": ["url", "body"]
                }),
            ),
        ]
    }
}

// ============================================================================
// 参数结构
// ============================================================================

/// 搜索参数
#[derive(Debug, serde::Deserialize)]
struct SearchParams {
    query: String,
}

/// 获取网页参数
#[derive(Debug, serde::Deserialize)]
struct FetchParams {
    url: String,
    max_chars: Option<usize>,
}

/// POST 请求参数
#[derive(Debug, serde::Deserialize)]
struct PostParams {
    url: String,
    body: String,
    content_type: Option<String>,
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 通用参数解析
fn parse_params<'a, T: serde::Deserialize<'a>>(params: &'a str) -> Result<T> {
    serde_json::from_str(params).map_err(|e| Error::Tool(format!("参数解析失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> KernelConfig {
        KernelConfig::from_json(r#"{"data_dir": "/tmp/peng_test"}"#).unwrap()
    }

    #[test]
    fn test_web_handler_domain() {
        let config = test_config();
        let handler = WebHandler::new(&config);
        assert_eq!(handler.domain(), ToolDomain::Web);
    }

    #[test]
    fn test_web_handler_tool_definitions() {
        let config = test_config();
        let handler = WebHandler::new(&config);
        let defs = handler.tool_definitions();
        assert_eq!(defs.len(), 3);
        let names: Vec<&str> = defs.iter().map(|d| d.function.name.as_str()).collect();
        assert!(names.contains(&"web_search"));
        assert!(names.contains(&"web_fetch"));
        assert!(names.contains(&"web_post"));
    }

    #[test]
    fn test_web_search_not_configured() {
        let config = test_config();
        let handler = WebHandler::new(&config);
        let result = handler.execute("web_search", r#"{"query": "test"}"#);
        assert!(result.is_ok());
        let output = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(json["status"], "not_configured");
    }

    #[test]
    fn test_web_handler_unknown_tool() {
        let config = test_config();
        let handler = WebHandler::new(&config);
        let result = handler.execute("unknown_tool", "{}");
        assert!(result.is_err());
    }
}
