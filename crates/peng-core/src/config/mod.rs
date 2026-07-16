//! peng-core: 内核配置模块
//!
//! 本模块定义了智能体内核的配置结构，支持以下配置加载方式：
//! - JSON 字符串解析
//! - 环境变量与 .env 文件（PENG_AGENT_* 前缀）
//! - 运行时动态修改单个配置项
//! - 序列化为 JSON 字符串

use crate::types::{Error, Result};

/// 内核配置结构
///
/// 包含智能体运行所需的全部配置项，涵盖数据目录、LLM 模型参数、
/// 工具执行限制、流式超时、技能目录、上下文压缩、工作模型、视觉模型等。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KernelConfig {
    // ---- 基础路径 ----
    /// 数据存储根目录
    pub data_dir: String,

    // ---- 主模型配置 ----
    /// 主模型名称（默认: deepseek-chat）
    #[serde(default = "default_model")]
    pub model: String,

    /// 主模型 API 基地址（默认: https://api.deepseek.com/v1）
    #[serde(default = "default_api_base")]
    pub api_base: String,

    /// 主模型 API 密钥
    #[serde(default)]
    pub api_key: String,

    /// 最大生成 token 数（默认: 4096）
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// 采样温度（默认: 0.7）
    #[serde(default = "default_temperature")]
    pub temperature: f64,

    /// 核采样概率阈值（默认: 0.9）
    #[serde(default = "default_top_p")]
    pub top_p: f64,

    // ---- 工具与执行限制 ----
    /// 单次对话最大工具调用轮数（默认: 50）
    #[serde(default = "default_max_tool_rounds")]
    pub max_tool_rounds: u32,

    /// 流式响应超时秒数（默认: 120）
    #[serde(default = "default_stream_timeout_secs")]
    pub stream_timeout_secs: u64,

    // ---- 技能 ----
    /// 技能定义文件目录
    #[serde(default)]
    pub skills_dir: String,

    // ---- 上下文压缩 ----
    /// 是否启用上下文压缩（默认: true）
    #[serde(default = "default_compression_enabled")]
    pub compression_enabled: bool,

    /// 触发压缩的上下文 token 阈值（默认: 4000）
    #[serde(default = "default_compression_threshold")]
    pub compression_threshold: u32,

    // ---- 工作模型（用于工具规划等辅助任务）----
    /// 工作模型名称
    #[serde(default)]
    pub worker_model: String,

    /// 工作模型 API 基地址
    #[serde(default)]
    pub worker_api_base: String,

    /// 工作模型 API 密钥
    #[serde(default)]
    pub worker_api_key: String,

    // ---- 视觉模型 ----
    /// 视觉模型名称
    #[serde(default)]
    pub vision_model: String,

    /// 视觉模型 API 基地址
    #[serde(default)]
    pub vision_api_base: String,

    /// 视觉模型 API 密钥
    #[serde(default)]
    pub vision_api_key: String,

    // ---- 容错 ----
    /// 工具执行出错时是否中止整个对话（默认: false）
    #[serde(default = "default_abort_on_error")]
    pub abort_on_error: bool,
}

// ---- 默认值函数 ----

fn default_model() -> String {
    "deepseek-chat".to_string()
}

fn default_api_base() -> String {
    "https://api.deepseek.com/v1".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f64 {
    0.7
}

fn default_top_p() -> f64 {
    0.9
}

fn default_max_tool_rounds() -> u32 {
    50
}

fn default_stream_timeout_secs() -> u64 {
    120
}

fn default_compression_enabled() -> bool {
    true
}

fn default_compression_threshold() -> u32 {
    4000
}

fn default_abort_on_error() -> bool {
    false
}

impl KernelConfig {
    /// 从 JSON 字符串解析配置
    ///
    /// # 参数
    /// - `json`: 包含配置数据的 JSON 字符串
    ///
    /// # 返回
    /// 解析成功返回 KernelConfig，失败返回 Error::Json
    ///
    /// # 示例
    /// ```ignore
    /// let config = KernelConfig::from_json(r#"{"data_dir": "/data/peng"}"#)?;
    /// ```
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| Error::Json(format!("配置 JSON 解析失败: {e}")))
    }

    /// 从环境变量与 .env 文件加载配置
    ///
    /// 读取 `data_dir` 路径下的 `.env` 文件，然后从环境变量中提取
    /// 以 `PENG_AGENT_` 为前缀的配置项。环境变量命名规则：
    /// 将配置字段名转为大写蛇形命名，加上 `PENG_AGENT_` 前缀。
    /// 例如 `model` → `PENG_AGENT_MODEL`，`api_base` → `PENG_AGENT_API_BASE`。
    ///
    /// # 参数
    /// - `data_dir`: 数据目录路径，同时作为 .env 文件的查找路径
    ///
    /// # 返回
    /// 加载成功返回 KernelConfig，失败返回相应错误
    pub fn from_env(data_dir: &str) -> Result<Self> {
        // 尝试加载 .env 文件
        let env_path = std::path::Path::new(data_dir).join(".env");
        if env_path.exists() {
            let content = std::fs::read_to_string(&env_path)
                .map_err(|e| Error::Io(format!("读取 .env 文件失败: {e}")))?;
            for line in content.lines() {
                let line = line.trim();
                // 跳过空行和注释
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    // 仅设置尚未存在的环境变量（不覆盖已有值）
                    if std::env::var(key).is_err() {
                        std::env::set_var(key, value);
                    }
                }
            }
        }

        // 从环境变量构建配置
        let mut config = Self {
            data_dir: data_dir.to_string(),
            model: env_or("PENG_AGENT_MODEL", &default_model()),
            api_base: env_or("PENG_AGENT_API_BASE", &default_api_base()),
            api_key: env_or("PENG_AGENT_API_KEY", ""),
            max_tokens: env_parse_or("PENG_AGENT_MAX_TOKENS", default_max_tokens()),
            temperature: env_parse_or("PENG_AGENT_TEMPERATURE", default_temperature()),
            top_p: env_parse_or("PENG_AGENT_TOP_P", default_top_p()),
            max_tool_rounds: env_parse_or("PENG_AGENT_MAX_TOOL_ROUNDS", default_max_tool_rounds()),
            stream_timeout_secs: env_parse_or("PENG_AGENT_STREAM_TIMEOUT_SECS", default_stream_timeout_secs()),
            skills_dir: env_or("PENG_AGENT_SKILLS_DIR", ""),
            compression_enabled: env_parse_or("PENG_AGENT_COMPRESSION_ENABLED", default_compression_enabled()),
            compression_threshold: env_parse_or("PENG_AGENT_COMPRESSION_THRESHOLD", default_compression_threshold()),
            worker_model: env_or("PENG_AGENT_WORKER_MODEL", ""),
            worker_api_base: env_or("PENG_AGENT_WORKER_API_BASE", ""),
            worker_api_key: env_or("PENG_AGENT_WORKER_API_KEY", ""),
            vision_model: env_or("PENG_AGENT_VISION_MODEL", ""),
            vision_api_base: env_or("PENG_AGENT_VISION_API_BASE", ""),
            vision_api_key: env_or("PENG_AGENT_VISION_API_KEY", ""),
            abort_on_error: env_parse_or("PENG_AGENT_ABORT_ON_ERROR", default_abort_on_error()),
        };

        // 如果环境变量指定了 data_dir，优先使用
        if let Ok(dir) = std::env::var("PENG_AGENT_DATA_DIR") {
            config.data_dir = dir;
        }

        Ok(config)
    }

    /// 运行时动态修改单个配置项
    ///
    /// 根据字段名（key）更新对应的配置值（value）。
    /// 数值类型字段会自动尝试解析字符串为对应类型。
    ///
    /// # 参数
    /// - `key`: 配置字段名（与结构体字段名一致）
    /// - `value`: 新的配置值（字符串形式）
    ///
    /// # 返回
    /// 修改成功返回 Ok(())，字段不存在或类型解析失败返回 Error
    pub fn set_config(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "data_dir" => self.data_dir = value.to_string(),
            "model" => self.model = value.to_string(),
            "api_base" => self.api_base = value.to_string(),
            "api_key" => self.api_key = value.to_string(),
            "max_tokens" => {
                self.max_tokens = value.parse().map_err(|_| {
                    Error::Other(format!("max_tokens 值无效: {value}"))
                })?
            }
            "temperature" => {
                self.temperature = value.parse().map_err(|_| {
                    Error::Other(format!("temperature 值无效: {value}"))
                })?
            }
            "top_p" => {
                self.top_p = value.parse().map_err(|_| {
                    Error::Other(format!("top_p 值无效: {value}"))
                })?
            }
            "max_tool_rounds" => {
                self.max_tool_rounds = value.parse().map_err(|_| {
                    Error::Other(format!("max_tool_rounds 值无效: {value}"))
                })?
            }
            "stream_timeout_secs" => {
                self.stream_timeout_secs = value.parse().map_err(|_| {
                    Error::Other(format!("stream_timeout_secs 值无效: {value}"))
                })?
            }
            "skills_dir" => self.skills_dir = value.to_string(),
            "compression_enabled" => {
                self.compression_enabled = value.parse().map_err(|_| {
                    Error::Other(format!("compression_enabled 值无效: {value}，应为 true/false"))
                })?
            }
            "compression_threshold" => {
                self.compression_threshold = value.parse().map_err(|_| {
                    Error::Other(format!("compression_threshold 值无效: {value}"))
                })?
            }
            "worker_model" => self.worker_model = value.to_string(),
            "worker_api_base" => self.worker_api_base = value.to_string(),
            "worker_api_key" => self.worker_api_key = value.to_string(),
            "vision_model" => self.vision_model = value.to_string(),
            "vision_api_base" => self.vision_api_base = value.to_string(),
            "vision_api_key" => self.vision_api_key = value.to_string(),
            "abort_on_error" => {
                self.abort_on_error = value.parse().map_err(|_| {
                    Error::Other(format!("abort_on_error 值无效: {value}，应为 true/false"))
                })?
            }
            _ => return Err(Error::Other(format!("未知配置项: {key}"))),
        }
        Ok(())
    }

    /// 将配置序列化为 JSON 字符串
    ///
    /// # 返回
    /// 格式化的 JSON 字符串
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| {
            format!("{{\"error\": \"配置序列化失败: {e}\"}}")
        })
    }
}

// ---- 环境变量辅助函数 ----

/// 读取环境变量，不存在时返回默认值
fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// 读取环境变量并解析为指定类型，失败时返回默认值
fn env_parse_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_json_minimal() {
        let json = r#"{"data_dir": "/tmp/peng"}"#;
        let config = KernelConfig::from_json(json).unwrap();
        assert_eq!(config.data_dir, "/tmp/peng");
        assert_eq!(config.model, "deepseek-chat");
        assert_eq!(config.api_base, "https://api.deepseek.com/v1");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.top_p, 0.9);
        assert_eq!(config.max_tool_rounds, 50);
        assert_eq!(config.stream_timeout_secs, 120);
        assert!(config.compression_enabled);
        assert_eq!(config.compression_threshold, 4000);
        assert!(!config.abort_on_error);
    }

    #[test]
    fn test_from_json_full() {
        let json = r#"{
            "data_dir": "/data/peng",
            "model": "gpt-4",
            "api_base": "https://api.openai.com/v1",
            "api_key": "sk-test",
            "max_tokens": 8192,
            "temperature": 0.5,
            "top_p": 0.95,
            "max_tool_rounds": 100,
            "stream_timeout_secs": 300,
            "skills_dir": "/skills",
            "compression_enabled": false,
            "compression_threshold": 8000,
            "worker_model": "gpt-3.5-turbo",
            "worker_api_base": "https://api.openai.com/v1",
            "worker_api_key": "sk-worker",
            "vision_model": "gpt-4-vision",
            "vision_api_base": "https://api.openai.com/v1",
            "vision_api_key": "sk-vision",
            "abort_on_error": true
        }"#;
        let config = KernelConfig::from_json(json).unwrap();
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.api_key, "sk-test");
        assert_eq!(config.max_tokens, 8192);
        assert!(!config.compression_enabled);
        assert!(config.abort_on_error);
    }

    #[test]
    fn test_set_config() {
        let mut config = KernelConfig::from_json(r#"{"data_dir": "/tmp"}"#).unwrap();
        config.set_config("model", "gpt-4").unwrap();
        assert_eq!(config.model, "gpt-4");
        config.set_config("max_tokens", "8192").unwrap();
        assert_eq!(config.max_tokens, 8192);
        config.set_config("compression_enabled", "false").unwrap();
        assert!(!config.compression_enabled);
        assert!(config.set_config("unknown_field", "value").is_err());
    }

    #[test]
    fn test_to_json_roundtrip() {
        let json = r#"{"data_dir": "/tmp/peng"}"#;
        let config = KernelConfig::from_json(json).unwrap();
        let output = config.to_json();
        let config2 = KernelConfig::from_json(&output).unwrap();
        assert_eq!(config.model, config2.model);
        assert_eq!(config.data_dir, config2.data_dir);
        assert_eq!(config.max_tokens, config2.max_tokens);
    }
}
