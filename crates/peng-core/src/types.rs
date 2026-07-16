//! peng-core: 鹏智能体核心类型定义
//!
//! 本模块定义了智能体运行所需的所有核心类型，包括：
//! - 错误类型与结果别名
//! - 聊天消息与工具调用结构
//! - 工具领域枚举
//! - 流式回调特征
//! - 内核状态结构

use std::fmt;

// ============================================================================
// 错误类型
// ============================================================================

/// 鹏智能体核心错误类型
///
/// 涵盖初始化、存储、LLM 调用、SQLite、工具执行、JSON 解析、IO 及其他错误场景。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// 内核已经初始化，不允许重复初始化
    #[error("内核已经初始化")]
    AlreadyInitialized,

    /// 内核尚未初始化，无法执行操作
    #[error("内核尚未初始化")]
    NotInitialized,

    /// 存储相关错误
    #[error("存储错误: {0}")]
    Storage(String),

    /// LLM API 调用相关错误
    #[error("LLM 错误: {0}")]
    Llm(String),

    /// SQLite 数据库相关错误
    #[error("SQLite 错误: {0}")]
    Sqlite(String),

    /// 工具执行相关错误
    #[error("工具错误: {0}")]
    Tool(String),

    /// JSON 序列化/反序列化错误
    #[error("JSON 错误: {0}")]
    Json(String),

    /// IO 操作相关错误
    #[error("IO 错误: {0}")]
    Io(String),

    /// 其他未分类错误
    #[error("其他错误: {0}")]
    Other(String),
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err.to_string())
    }
}

/// 鹏智能体统一结果类型
pub type Result<T> = std::result::Result<T, Error>;

// ============================================================================
// 工具调用相关结构
// ============================================================================

/// 函数调用描述
///
/// 包含函数名称和 JSON 格式的参数字符串。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionCall {
    /// 函数名称
    pub name: String,
    /// 函数参数（JSON 字符串）
    pub arguments: String,
}

/// 工具调用请求
///
/// LLM 返回的工具调用指令，包含唯一标识和函数调用详情。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    /// 工具调用的唯一标识符
    pub id: String,
    /// 函数调用详情
    pub function: FunctionCall,
}

/// 工具执行结果
///
/// 工具执行完毕后返回给 LLM 的结果数据。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolResult {
    /// 对应的工具调用标识符
    pub tool_call_id: String,
    /// 工具执行结果内容
    pub content: String,
}

// ============================================================================
// 聊天消息
// ============================================================================

/// 聊天消息结构
///
/// 与 LLM API 交互的标准消息格式，支持系统消息、用户消息、
/// 助手消息（含工具调用）以及工具结果消息。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    /// 消息角色：system / user / assistant / tool
    pub role: String,
    /// 消息文本内容（工具结果消息时可能为空）
    pub content: Option<String>,
    /// 助手消息中的工具调用列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// 工具结果消息对应的工具调用标识符
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    /// 创建系统消息
    ///
    /// # 参数
    /// - `content`: 系统提示词内容
    ///
    /// # 返回
    /// 角色为 "system" 的聊天消息
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content.to_string()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// 创建用户消息
    ///
    /// # 参数
    /// - `content`: 用户输入内容
    ///
    /// # 返回
    /// 角色为 "user" 的聊天消息
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.to_string()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// 创建助手消息（纯文本，无工具调用）
    ///
    /// # 参数
    /// - `content`: 助手回复内容
    ///
    /// # 返回
    /// 角色为 "assistant" 的聊天消息
    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: Some(content.to_string()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// 创建带工具调用的助手消息
    ///
    /// # 参数
    /// - `content`: 助手回复文本（可为空）
    /// - `tool_calls`: 工具调用列表
    ///
    /// # 返回
    /// 角色为 "assistant" 且包含工具调用的聊天消息
    pub fn assistant_with_tools(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// 创建工具结果消息
    ///
    /// # 参数
    /// - `tool_call_id`: 对应的工具调用标识符
    /// - `content`: 工具执行结果内容
    ///
    /// # 返回
    /// 角色为 "tool" 的聊天消息
    pub fn tool_result(tool_call_id: &str, content: &str) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content.to_string()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
        }
    }
}

// ============================================================================
// 工具领域枚举
// ============================================================================

/// 工具领域分类
///
/// 将智能体可调用的工具划分为六大领域，便于路由和管理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ToolDomain {
    /// 代码领域：代码生成、编辑、执行
    Code,
    /// 媒体领域：图片、音频、视频处理
    Media,
    /// 文件领域：文件读写、目录操作
    File,
    /// 手机领域：设备控制、系统设置
    Phone,
    /// 网页领域：浏览器、网络请求
    Web,
    /// 记忆领域：知识存储、检索
    Memory,
}

impl fmt::Display for ToolDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolDomain::Code => write!(f, "代码"),
            ToolDomain::Media => write!(f, "媒体"),
            ToolDomain::File => write!(f, "文件"),
            ToolDomain::Phone => write!(f, "手机"),
            ToolDomain::Web => write!(f, "网页"),
            ToolDomain::Memory => write!(f, "记忆"),
        }
    }
}

// ============================================================================
// 流式回调特征
// ============================================================================

/// 流式输出回调特征
///
/// 用于在 LLM 流式响应和工具执行过程中向调用方推送实时事件。
/// 实现此特征即可接收 token 级别的流式输出、工具执行状态和完成/错误通知。
pub trait StreamCallback: Send + Sync {
    /// 收到一个流式 token
    ///
    /// # 参数
    /// - `token`: LLM 输出的单个 token 文本
    fn on_token(&self, token: &str);

    /// 工具开始执行
    ///
    /// # 参数
    /// - `name`: 工具名称
    /// - `args`: 工具参数（JSON 字符串）
    fn on_tool_start(&self, name: &str, args: &str);

    /// 工具执行完毕
    ///
    /// # 参数
    /// - `name`: 工具名称
    /// - `result`: 工具执行结果
    fn on_tool_end(&self, name: &str, result: &str);

    /// 流式响应完成
    ///
    /// # 参数
    /// - `response`: 完整的 LLM 响应文本
    fn on_complete(&self, response: &str);

    /// 发生错误
    ///
    /// # 参数
    /// - `error`: 错误描述信息
    fn on_error(&self, error: &str);
}

// ============================================================================
// 内核状态
// ============================================================================

/// 内核运行状态
///
/// 描述智能体内核的当前初始化状态、使用的模型及状态消息。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KernelStatus {
    /// 是否已初始化
    pub initialized: bool,
    /// 当前使用的模型名称
    pub model: String,
    /// 状态描述消息
    pub message: String,
}

impl KernelStatus {
    /// 创建未初始化状态
    pub fn uninitialized() -> Self {
        Self {
            initialized: false,
            model: String::new(),
            message: "内核尚未初始化".to_string(),
        }
    }

    /// 创建已初始化状态
    pub fn initialized(model: &str) -> Self {
        Self {
            initialized: true,
            model: model.to_string(),
            message: "内核已就绪".to_string(),
        }
    }
}
