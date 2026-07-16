//! 代码领域工具处理器
//!
//! 提供代码执行相关工具：
//! - `execute_shell` — 执行 shell 命令
//! - `execute_python` — 执行 Python 代码
//! - `execute_node` — 执行 Node.js 代码
//!
//! 在 Android/Termux 环境下这些工具可以直接工作，
//! 因为 Termux 提供了 sh、python3、node 等运行时。

use std::time::Duration;

use crate::config::KernelConfig;
use crate::llm::ToolDefinition;
use crate::types::{Error, Result, ToolDomain};

use super::router::ToolHandler;

// ============================================================================
// CodeHandler
// ============================================================================

/// 代码领域工具处理器
///
/// 通过 `tokio::process::Command` 执行外部命令。
/// 所有命令均在子进程中运行，支持超时控制。
pub struct CodeHandler {
    /// 默认命令执行超时（秒）
    default_timeout: u64,
}

impl CodeHandler {
    /// 创建代码领域处理器
    ///
    /// # 参数
    /// - `config`: 内核配置（从中读取超时等参数）
    pub fn new(config: &KernelConfig) -> Self {
        Self {
            default_timeout: config.stream_timeout_secs,
        }
    }

    /// 执行 shell 命令
    ///
    /// # 参数
    /// - `params`: JSON 字符串，包含 `command` 和可选的 `timeout` 字段
    ///
    /// # 返回
    /// 命令的 stdout + stderr 合并输出
    fn execute_shell(&self, params: &str) -> Result<String> {
        let p: ShellParams = parse_params(params)?;

        let timeout_secs = p.timeout.unwrap_or(self.default_timeout);
        let command = p.command;

        // 使用 tokio runtime 执行异步命令
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Tool(format!("创建 tokio runtime 失败: {e}")))?;

        rt.block_on(async {
            let result = tokio::time::timeout(
                Duration::from_secs(timeout_secs),
                tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&command)
                    .output(),
            )
            .await;

            match result {
                Ok(Ok(output)) => {
                    let mut combined = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        if !combined.is_empty() {
                            combined.push('\n');
                        }
                        combined.push_str(&stderr);
                    }
                    if output.status.success() {
                        Ok(combined)
                    } else {
                        Ok(format!(
                            "退出码: {}\n{}",
                            output.status.code().unwrap_or(-1),
                            combined
                        ))
                    }
                }
                Ok(Err(e)) => Err(Error::Tool(format!("Shell 命令执行失败: {e}"))),
                Err(_) => Err(Error::Tool(format!(
                    "Shell 命令执行超时（{}秒）",
                    timeout_secs
                ))),
            }
        })
    }

    /// 执行 Python 代码
    ///
    /// 通过 `python3 -c` 或 `python -c` 执行代码片段。
    ///
    /// # 参数
    /// - `params`: JSON 字符串，包含 `code` 字段
    fn execute_python(&self, params: &str) -> Result<String> {
        let p: CodeParams = parse_params(params)?;
        let code = p.code;

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Tool(format!("创建 tokio runtime 失败: {e}")))?;

        rt.block_on(async {
            // 先尝试 python3，再回退到 python
            let result = tokio::time::timeout(
                Duration::from_secs(self.default_timeout),
                async {
                    let output = tokio::process::Command::new("python3")
                        .arg("-c")
                        .arg(&code)
                        .output()
                        .await;

                    // 如果 python3 不存在，尝试 python
                    match output {
                        Ok(out) => Ok(out),
                        Err(_) => {
                            tokio::process::Command::new("python")
                                .arg("-c")
                                .arg(&code)
                                .output()
                                .await
                        }
                    }
                },
            )
            .await;

            match result {
                Ok(Ok(output)) => {
                    let mut combined = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        if !combined.is_empty() {
                            combined.push('\n');
                        }
                        combined.push_str(&stderr);
                    }
                    if output.status.success() {
                        Ok(combined)
                    } else {
                        Ok(format!(
                            "退出码: {}\n{}",
                            output.status.code().unwrap_or(-1),
                            combined
                        ))
                    }
                }
                Ok(Err(e)) => Err(Error::Tool(format!(
                    "Python 执行失败（python3 和 python 均不可用）: {e}"
                ))),
                Err(_) => Err(Error::Tool(format!(
                    "Python 执行超时（{}秒）",
                    self.default_timeout
                ))),
            }
        })
    }

    /// 执行 Node.js 代码
    ///
    /// 通过 `node -e` 执行 JavaScript 代码片段。
    ///
    /// # 参数
    /// - `params`: JSON 字符串，包含 `code` 字段
    fn execute_node(&self, params: &str) -> Result<String> {
        let p: CodeParams = parse_params(params)?;
        let code = p.code;

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Tool(format!("创建 tokio runtime 失败: {e}")))?;

        rt.block_on(async {
            let result = tokio::time::timeout(
                Duration::from_secs(self.default_timeout),
                tokio::process::Command::new("node")
                    .arg("-e")
                    .arg(&code)
                    .output(),
            )
            .await;

            match result {
                Ok(Ok(output)) => {
                    let mut combined = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        if !combined.is_empty() {
                            combined.push('\n');
                        }
                        combined.push_str(&stderr);
                    }
                    if output.status.success() {
                        Ok(combined)
                    } else {
                        Ok(format!(
                            "退出码: {}\n{}",
                            output.status.code().unwrap_or(-1),
                            combined
                        ))
                    }
                }
                Ok(Err(e)) => Err(Error::Tool(format!("Node.js 执行失败: {e}"))),
                Err(_) => Err(Error::Tool(format!(
                    "Node.js 执行超时（{}秒）",
                    self.default_timeout
                ))),
            }
        })
    }
}

impl ToolHandler for CodeHandler {
    fn domain(&self) -> ToolDomain {
        ToolDomain::Code
    }

    fn execute(&self, tool_name: &str, params: &str) -> Result<String> {
        match tool_name {
            "execute_shell" => self.execute_shell(params),
            "execute_python" => self.execute_python(params),
            "execute_node" => self.execute_node(params),
            _ => Err(Error::Tool(format!(
                "代码领域未知工具: {tool_name}"
            ))),
        }
    }

    fn tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition::function(
                "execute_shell",
                "执行 shell 命令。在子进程中运行命令并返回输出。在 Android/Termux 环境下可直接使用。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "要执行的 shell 命令"
                        },
                        "timeout": {
                            "type": "integer",
                            "description": "超时时间（秒），默认使用全局配置的超时值",
                            "default": 30
                        }
                    },
                    "required": ["command"]
                }),
            ),
            ToolDefinition::function(
                "execute_python",
                "执行 Python 代码片段。通过 python3 -c 或 python -c 运行。适用于数据处理、算法执行等场景。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "要执行的 Python 代码"
                        }
                    },
                    "required": ["code"]
                }),
            ),
            ToolDefinition::function(
                "execute_node",
                "执行 Node.js (JavaScript) 代码片段。通过 node -e 运行。适用于异步操作、npm 包调用等场景。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "要执行的 JavaScript 代码"
                        }
                    },
                    "required": ["code"]
                }),
            ),
        ]
    }
}

// ============================================================================
// 参数结构
// ============================================================================

/// shell 命令参数
#[derive(Debug, serde::Deserialize)]
struct ShellParams {
    /// 要执行的 shell 命令
    command: String,
    /// 超时时间（秒）
    timeout: Option<u64>,
}

/// 代码执行参数（Python / Node）
#[derive(Debug, serde::Deserialize)]
struct CodeParams {
    /// 要执行的代码
    code: String,
}

/// 通用参数解析辅助函数
fn parse_params<'a, T: serde::Deserialize<'a>>(params: &'a str) -> Result<T> {
    serde_json::from_str(params)
        .map_err(|e| Error::Tool(format!("参数解析失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> KernelConfig {
        KernelConfig::from_json(r#"{"data_dir": "/tmp/peng_test"}"#).unwrap()
    }

    #[test]
    fn test_code_handler_domain() {
        let config = test_config();
        let handler = CodeHandler::new(&config);
        assert_eq!(handler.domain(), ToolDomain::Code);
    }

    #[test]
    fn test_code_handler_tool_definitions() {
        let config = test_config();
        let handler = CodeHandler::new(&config);
        let defs = handler.tool_definitions();
        assert_eq!(defs.len(), 3);
        let names: Vec<&str> = defs.iter().map(|d| d.function.name.as_str()).collect();
        assert!(names.contains(&"execute_shell"));
        assert!(names.contains(&"execute_python"));
        assert!(names.contains(&"execute_node"));
    }

    #[test]
    fn test_code_handler_unknown_tool() {
        let config = test_config();
        let handler = CodeHandler::new(&config);
        let result = handler.execute("unknown_tool", "{}");
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_shell_echo() {
        let config = test_config();
        let handler = CodeHandler::new(&config);
        let result = handler.execute("execute_shell", r#"{"command": "echo hello"}"#);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("hello"));
    }

    #[test]
    fn test_execute_shell_invalid_params() {
        let config = test_config();
        let handler = CodeHandler::new(&config);
        let result = handler.execute("execute_shell", "not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_shell_timeout() {
        let config = test_config();
        let handler = CodeHandler::new(&config);
        let result = handler.execute("execute_shell", r#"{"command": "sleep 10", "timeout": 1}"#);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Tool(msg) => assert!(msg.contains("超时")),
            _ => panic!("期望超时错误"),
        }
    }
}
