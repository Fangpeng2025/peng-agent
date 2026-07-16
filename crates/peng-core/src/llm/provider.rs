//! LLM 提供商检测与配置
//!
//! 根据端点 URL 自动识别 LLM 提供商，并提供对应的请求头、
//! 认证方式和 API 路径适配。

/// LLM 提供商枚举
///
/// 支持的提供商：DeepSeek、OpenAI、智谱、讯飞、Anthropic 及自定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    /// DeepSeek（深度求索）
    DeepSeek,
    /// OpenAI
    OpenAI,
    /// 智谱 AI（BigModel / ZhipuAI）
    Zhipu,
    /// 讯飞星火（XFYun）
    Xfyun,
    /// Anthropic（Claude）
    Anthropic,
    /// 自定义提供商
    Custom,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::DeepSeek => write!(f, "DeepSeek"),
            Provider::OpenAI => write!(f, "OpenAI"),
            Provider::Zhipu => write!(f, "Zhipu"),
            Provider::Xfyun => write!(f, "Xfyun"),
            Provider::Anthropic => write!(f, "Anthropic"),
            Provider::Custom => write!(f, "Custom"),
        }
    }
}

/// 根据端点 URL 自动检测 LLM 提供商
///
/// # 匹配规则
/// - URL 包含 `deepseek` → DeepSeek
/// - URL 包含 `openai.com` → OpenAI
/// - URL 包含 `bigmodel.cn` 或 `zhipuai` → Zhipu
/// - URL 包含 `xfyun.cn` → Xfyun
/// - URL 包含 `anthropic.com` → Anthropic
/// - 其他 → Custom
///
/// # 参数
/// - `url`: API 端点 URL
///
/// # 返回
/// 检测到的提供商类型
pub fn detect_provider(url: &str) -> Provider {
    let lower = url.to_lowercase();
    if lower.contains("deepseek") {
        Provider::DeepSeek
    } else if lower.contains("openai.com") {
        Provider::OpenAI
    } else if lower.contains("bigmodel.cn") || lower.contains("zhipuai") {
        Provider::Zhipu
    } else if lower.contains("xfyun.cn") {
        Provider::Xfyun
    } else if lower.contains("anthropic.com") {
        Provider::Anthropic
    } else {
        Provider::Custom
    }
}

impl Provider {
    /// 构建完整的 chat completions 请求 URL
    ///
    /// 确保端点 URL 以 `/chat/completions` 结尾。
    /// 对于 Anthropic，使用 `/v1/messages` 路径。
    ///
    /// # 参数
    /// - `endpoint`: 基础 API 端点 URL
    ///
    /// # 返回
    /// 完整的请求 URL
    pub fn base_url(&self, endpoint: &str) -> String {
        let trimmed = endpoint.trim_end_matches('/');

        match self {
            Provider::Anthropic => {
                // Anthropic 使用 /v1/messages 而非 /chat/completions
                if trimmed.ends_with("/v1/messages") {
                    trimmed.to_string()
                } else if trimmed.ends_with("/v1") {
                    format!("{trimmed}/messages")
                } else {
                    format!("{trimmed}/v1/messages")
                }
            }
            _ => {
                // OpenAI 兼容格式：确保以 /chat/completions 结尾
                if trimmed.ends_with("/chat/completions") {
                    trimmed.to_string()
                } else if trimmed.ends_with("/v1") {
                    format!("{trimmed}/chat/completions")
                } else if trimmed.ends_with("/v1/") {
                    format!("{}chat/completions", trimmed.trim_end_matches('/'))
                } else {
                    format!("{trimmed}/chat/completions")
                }
            }
        }
    }

    /// 构建认证请求头
    ///
    /// 大多数提供商使用 `Authorization: Bearer <key>` 格式，
    /// Anthropic 使用 `x-api-key: <key>` 格式。
    ///
    /// # 参数
    /// - `api_key`: API 密钥
    ///
    /// # 返回
    /// (header_name, header_value) 元组
    pub fn auth_header(&self, api_key: &str) -> (String, String) {
        match self {
            Provider::Anthropic => ("x-api-key".to_string(), api_key.to_string()),
            _ => (
                "Authorization".to_string(),
                format!("Bearer {api_key}"),
            ),
        }
    }

    /// 获取 Anthropic API 版本号
    ///
    /// 仅 Anthropic 提供商返回版本号，其他返回 None。
    ///
    /// # 返回
    /// Anthropic 返回 `"2023-06-01"`，其他返回 `None`
    pub fn anthropic_version(&self) -> Option<&str> {
        match self {
            Provider::Anthropic => Some("2023-06-01"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_deepseek() {
        assert_eq!(detect_provider("https://api.deepseek.com/v1"), Provider::DeepSeek);
        assert_eq!(detect_provider("https://DEEPSEEK.io/api"), Provider::DeepSeek);
    }

    #[test]
    fn test_detect_openai() {
        assert_eq!(detect_provider("https://api.openai.com/v1"), Provider::OpenAI);
    }

    #[test]
    fn test_detect_zhipu() {
        assert_eq!(detect_provider("https://open.bigmodel.cn/api/paas/v4"), Provider::Zhipu);
        assert_eq!(detect_provider("https://zhipuai.cn/api"), Provider::Zhipu);
    }

    #[test]
    fn test_detect_xfyun() {
        assert_eq!(detect_provider("https://spark-api.xfyun.cn/v1"), Provider::Xfyun);
    }

    #[test]
    fn test_detect_anthropic() {
        assert_eq!(detect_provider("https://api.anthropic.com/v1"), Provider::Anthropic);
    }

    #[test]
    fn test_detect_custom() {
        assert_eq!(detect_provider("https://my-llm.example.com/api"), Provider::Custom);
    }

    #[test]
    fn test_base_url_openai_compatible() {
        let p = Provider::DeepSeek;
        assert_eq!(
            p.base_url("https://api.deepseek.com/v1"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            p.base_url("https://api.deepseek.com/v1/"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        // Already has /chat/completions
        assert_eq!(
            p.base_url("https://api.deepseek.com/v1/chat/completions"),
            "https://api.deepseek.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_base_url_anthropic() {
        let p = Provider::Anthropic;
        assert_eq!(
            p.base_url("https://api.anthropic.com/v1"),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            p.base_url("https://api.anthropic.com/v1/messages"),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn test_auth_header_bearer() {
        let p = Provider::DeepSeek;
        let (name, value) = p.auth_header("sk-test123");
        assert_eq!(name, "Authorization");
        assert_eq!(value, "Bearer sk-test123");
    }

    #[test]
    fn test_auth_header_anthropic() {
        let p = Provider::Anthropic;
        let (name, value) = p.auth_header("sk-ant-test");
        assert_eq!(name, "x-api-key");
        assert_eq!(value, "sk-ant-test");
    }

    #[test]
    fn test_anthropic_version() {
        assert_eq!(Provider::Anthropic.anthropic_version(), Some("2023-06-01"));
        assert_eq!(Provider::DeepSeek.anthropic_version(), None);
        assert_eq!(Provider::OpenAI.anthropic_version(), None);
    }
}
