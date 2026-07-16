//! 手机领域工具处理器
//!
//! 提供设备控制和系统设置相关工具：
//! - `send_notification` — 发送通知
//! - `set_clipboard` — 设置剪贴板内容
//! - `get_clipboard` — 获取剪贴板内容
//! - `take_screenshot` — 截屏
//! - `perform_gesture` — 执行手势操作
//! - `accessibility_action` — 无障碍操作
//! - `make_call` — 拨打电话
//! - `send_sms` — 发送短信
//! - `open_app` — 打开应用
//! - `set_brightness` — 设置屏幕亮度
//! - `set_volume` — 设置音量
//!
//! 这些工具全部需要 Kotlin 回调才能实际执行。
//! Rust 层返回特殊的 JSON 标记 `{"status": "kotlin_callback_required", ...}`，
//! JNI 层会拦截此标记并回调到 Kotlin 侧执行。
//! 这是 "SuspendingJNI" 模式的核心设计。

use crate::config::KernelConfig;
use crate::llm::ToolDefinition;
use crate::types::{Result, ToolDomain};

use super::router::ToolHandler;

// ============================================================================
// PhoneHandler
// ============================================================================

/// 手机领域工具处理器
///
/// 所有工具均返回 Kotlin 回调标记，不在 Rust 侧执行任何实际操作。
/// JNI 层拦截 `kotlin_callback_required` 状态后，将调用挂起并回调到 Kotlin 侧。
pub struct PhoneHandler;

impl PhoneHandler {
    /// 创建手机领域处理器
    pub fn new(_config: &KernelConfig) -> Self {
        Self
    }

    /// 生成 Kotlin 回调标记 JSON
    ///
    /// # 参数
    /// - `tool`: 工具名称
    /// - `params`: 原始参数 JSON 字符串
    ///
    /// # 返回
    /// 包含 `kotlin_callback_required` 状态的 JSON 字符串
    fn kotlin_callback_required(tool: &str, params: &str) -> Result<String> {
        Ok(serde_json::json!({
            "status": "kotlin_callback_required",
            "tool": tool,
            "params": params
        })
        .to_string())
    }
}

impl ToolHandler for PhoneHandler {
    fn domain(&self) -> ToolDomain {
        ToolDomain::Phone
    }

    fn execute(&self, tool_name: &str, params: &str) -> Result<String> {
        match tool_name {
            "send_notification"
            | "set_clipboard"
            | "get_clipboard"
            | "take_screenshot"
            | "perform_gesture"
            | "accessibility_action"
            | "make_call"
            | "send_sms"
            | "open_app"
            | "set_brightness"
            | "set_volume" => Self::kotlin_callback_required(tool_name, params),
            _ => Err(crate::types::Error::Tool(format!(
                "手机领域未知工具: {tool_name}"
            ))),
        }
    }

    fn tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition::function(
                "send_notification",
                "发送系统通知。需要 Kotlin 回调执行。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "通知标题"
                        },
                        "content": {
                            "type": "string",
                            "description": "通知内容"
                        },
                        "channel_id": {
                            "type": "string",
                            "description": "通知渠道 ID",
                            "default": "default"
                        }
                    },
                    "required": ["title", "content"]
                }),
            ),
            ToolDefinition::function(
                "set_clipboard",
                "设置系统剪贴板内容。需要 Kotlin 回调执行。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "要复制到剪贴板的文本"
                        }
                    },
                    "required": ["text"]
                }),
            ),
            ToolDefinition::function(
                "get_clipboard",
                "获取系统剪贴板内容。需要 Kotlin 回调执行。",
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
            ToolDefinition::function(
                "take_screenshot",
                "截取屏幕截图。需要 Kotlin 回调执行。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "save_path": {
                            "type": "string",
                            "description": "截图保存路径（可选，默认保存到数据目录）"
                        }
                    }
                }),
            ),
            ToolDefinition::function(
                "perform_gesture",
                "执行手势操作（点击、滑动、长按等）。需要 Kotlin 回调执行。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "gesture_type": {
                            "type": "string",
                            "enum": ["tap", "long_press", "swipe", "pinch", "rotate"],
                            "description": "手势类型"
                        },
                        "x": {
                            "type": "integer",
                            "description": "起始 X 坐标"
                        },
                        "y": {
                            "type": "integer",
                            "description": "起始 Y 坐标"
                        },
                        "x2": {
                            "type": "integer",
                            "description": "结束 X 坐标（用于 swipe）"
                        },
                        "y2": {
                            "type": "integer",
                            "description": "结束 Y 坐标（用于 swipe）"
                        },
                        "duration_ms": {
                            "type": "integer",
                            "description": "手势持续时间（毫秒）",
                            "default": 300
                        }
                    },
                    "required": ["gesture_type", "x", "y"]
                }),
            ),
            ToolDefinition::function(
                "accessibility_action",
                "执行无障碍操作（点击 UI 元素、输入文本等）。需要 Kotlin 回调执行。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["click", "long_click", "type_text", "scroll_forward", "scroll_backward", "focus"],
                            "description": "无障碍操作类型"
                        },
                        "node_id": {
                            "type": "string",
                            "description": "目标 UI 节点 ID（可选，通过 accessibility tree 获取）"
                        },
                        "text": {
                            "type": "string",
                            "description": "要输入的文本（用于 type_text 操作）"
                        }
                    },
                    "required": ["action"]
                }),
            ),
            ToolDefinition::function(
                "make_call",
                "拨打电话。需要 Kotlin 回调执行。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "number": {
                            "type": "string",
                            "description": "电话号码"
                        }
                    },
                    "required": ["number"]
                }),
            ),
            ToolDefinition::function(
                "send_sms",
                "发送短信。需要 Kotlin 回调执行。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "number": {
                            "type": "string",
                            "description": "接收方电话号码"
                        },
                        "message": {
                            "type": "string",
                            "description": "短信内容"
                        }
                    },
                    "required": ["number", "message"]
                }),
            ),
            ToolDefinition::function(
                "open_app",
                "打开指定应用。需要 Kotlin 回调执行。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "package_name": {
                            "type": "string",
                            "description": "应用包名（如 com.android.settings）"
                        }
                    },
                    "required": ["package_name"]
                }),
            ),
            ToolDefinition::function(
                "set_brightness",
                "设置屏幕亮度。需要 Kotlin 回调执行。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "level": {
                            "type": "integer",
                            "description": "亮度级别（0-255）",
                            "minimum": 0,
                            "maximum": 255
                        }
                    },
                    "required": ["level"]
                }),
            ),
            ToolDefinition::function(
                "set_volume",
                "设置系统音量。需要 Kotlin 回调执行。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "stream_type": {
                            "type": "string",
                            "enum": ["music", "ring", "notification", "alarm", "system"],
                            "description": "音频流类型",
                            "default": "music"
                        },
                        "level": {
                            "type": "integer",
                            "description": "音量级别（0-最大值，不同流类型最大值不同）",
                            "minimum": 0
                        }
                    },
                    "required": ["level"]
                }),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> KernelConfig {
        KernelConfig::from_json(r#"{"data_dir": "/tmp/peng_test"}"#).unwrap()
    }

    #[test]
    fn test_phone_handler_domain() {
        let config = test_config();
        let handler = PhoneHandler::new(&config);
        assert_eq!(handler.domain(), ToolDomain::Phone);
    }

    #[test]
    fn test_phone_handler_tool_definitions() {
        let config = test_config();
        let handler = PhoneHandler::new(&config);
        let defs = handler.tool_definitions();
        assert_eq!(defs.len(), 11);
    }

    #[test]
    fn test_phone_handler_returns_kotlin_callback() {
        let config = test_config();
        let handler = PhoneHandler::new(&config);

        let tools = [
            "send_notification",
            "set_clipboard",
            "get_clipboard",
            "take_screenshot",
            "perform_gesture",
            "accessibility_action",
            "make_call",
            "send_sms",
            "open_app",
            "set_brightness",
            "set_volume",
        ];

        for tool in &tools {
            let result = handler.execute(tool, r#"{"test": true}"#);
            assert!(result.is_ok(), "工具 {tool} 应该返回 Ok");
            let output = result.unwrap();
            let json: serde_json::Value = serde_json::from_str(&output).unwrap();
            assert_eq!(json["status"], "kotlin_callback_required", "工具 {tool} 应返回 kotlin_callback_required");
            assert_eq!(json["tool"], *tool);
        }
    }

    #[test]
    fn test_phone_handler_unknown_tool() {
        let config = test_config();
        let handler = PhoneHandler::new(&config);
        let result = handler.execute("unknown_tool", "{}");
        assert!(result.is_err());
    }
}
