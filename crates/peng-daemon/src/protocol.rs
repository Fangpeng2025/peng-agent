//! IPC Protocol - 请求/响应定义

use serde::{Deserialize, Serialize};

/// IPC 请求
#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

/// IPC 响应
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<StreamEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn event(event: StreamEvent) -> Self {
        Self {
            id: 0,
            event: Some(event),
            result: None,
            error: None,
        }
    }
    
    pub fn result(id: u64, result: serde_json::Value) -> Self {
        Self {
            id,
            event: None,
            result: Some(result),
            error: None,
        }
    }
    
    pub fn error(id: u64, error: String) -> Self {
        Self {
            id,
            event: None,
            result: None,
            error: Some(error),
        }
    }
}

/// 流式事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    Token { data: String },
    ToolStart { name: String, args: String },
    ToolEnd { name: String, result: String },
    PhoneCallback { callback_id: String, tool: String, params: serde_json::Value },
    Complete { data: String },
    Error { message: String },
}