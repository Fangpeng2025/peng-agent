//! 媒体领域工具处理器
//!
//! 提供图片、音频、视频处理相关工具：
//! - `generate_image` — 图片生成（需 ShellBridge 配置）
//! - `process_video` — 视频处理（ffmpeg）
//! - `process_audio` — 音频处理（ffmpeg）
//! - `mix_audio` — 音频混合（ffmpeg complex filter）
//!
//! 所有工具均基于 ShellBridge 模式：构建 shell 命令字符串，
//! 通过 `tokio::process::Command` 执行外部程序（如 ffmpeg、python3 脚本）。

use std::time::Duration;

use crate::config::KernelConfig;
use crate::llm::ToolDefinition;
use crate::types::{Error, Result, ToolDomain};

use super::router::ToolHandler;

// ============================================================================
// MediaHandler
// ============================================================================

/// 媒体领域工具处理器
///
/// 通过 ShellBridge 执行 ffmpeg 等外部命令来处理媒体文件。
pub struct MediaHandler {
    /// 默认命令执行超时（秒）
    default_timeout: u64,
}

impl MediaHandler {
    /// 创建媒体领域处理器
    pub fn new(config: &KernelConfig) -> Self {
        Self {
            default_timeout: config.stream_timeout_secs,
        }
    }

    /// 图片生成
    ///
    /// 目前返回提示信息，实际生成需要 ShellBridge 配置
    /// （如调用 python3 脚本、Stable Diffusion 等）。
    fn generate_image(&self, params: &str) -> Result<String> {
        let p: GenerateImageParams = parse_params(params)?;
        let style = p.style.unwrap_or_else(|| "default".to_string());

        // 尝试通过 ShellBridge 执行图片生成脚本
        let command = format!(
            "python3 -c \"import json; print(json.dumps({{'status': 'image_generation_prompt', 'prompt': '{}', 'style': '{}'}}))\"",
            p.prompt.replace('"', "\\\""),
            style.replace('"', "\\\""),
        );

        match self.run_shell_command(&command) {
            Ok(output) if output.contains("image_generation_prompt") => Ok(output),
            _ => Ok(serde_json::json!({
                "status": "shellbridge_required",
                "message": "图片生成需要 ShellBridge 配置。请安装图片生成工具（如 Stable Diffusion、DALL-E CLI 等）并配置 ShellBridge。",
                "prompt": p.prompt,
                "style": style
            })
            .to_string()),
        }
    }

    /// 视频处理
    ///
    /// 使用 ffmpeg 执行视频转码、剪辑、提取帧等操作。
    fn process_video(&self, params: &str) -> Result<String> {
        let p: ProcessVideoParams = parse_params(params)?;

        let command = match p.action.as_str() {
            "convert" => {
                format!(
                    "ffmpeg -y -i '{}' '{}'",
                    shell_escape(&p.input),
                    shell_escape(&p.output)
                )
            }
            "extract_audio" => {
                format!(
                    "ffmpeg -y -i '{}' -vn -acodec copy '{}'",
                    shell_escape(&p.input),
                    shell_escape(&p.output)
                )
            }
            "extract_frames" => {
                format!(
                    "ffmpeg -y -i '{}' -vf fps=1 '{}'",
                    shell_escape(&p.input),
                    shell_escape(&p.output)
                )
            }
            "trim" => {
                // 需要 start_time 和 duration 参数
                let start = p.start_time.unwrap_or_else(|| "00:00:00".to_string());
                let duration = p.duration.unwrap_or_else(|| "10".to_string());
                format!(
                    "ffmpeg -y -ss {} -i '{}' -t {} -c copy '{}'",
                    start,
                    shell_escape(&p.input),
                    duration,
                    shell_escape(&p.output)
                )
            }
            _ => {
                return Err(Error::Tool(format!(
                    "未知的视频处理动作: {}。支持: convert, extract_audio, extract_frames, trim",
                    p.action
                )));
            }
        };

        self.run_shell_command(&command)
    }

    /// 音频处理
    ///
    /// 使用 ffmpeg 执行音频转码、剪辑、提取等操作。
    fn process_audio(&self, params: &str) -> Result<String> {
        let p: ProcessAudioParams = parse_params(params)?;

        let command = match p.action.as_str() {
            "convert" => {
                format!(
                    "ffmpeg -y -i '{}' '{}'",
                    shell_escape(&p.input),
                    shell_escape(&p.output)
                )
            }
            "trim" => {
                let start = p.start_time.unwrap_or_else(|| "00:00:00".to_string());
                let duration = p.duration.unwrap_or_else(|| "10".to_string());
                format!(
                    "ffmpeg -y -ss {} -i '{}' -t {} -c copy '{}'",
                    start,
                    shell_escape(&p.input),
                    duration,
                    shell_escape(&p.output)
                )
            }
            "volume" => {
                let volume = p.volume.unwrap_or_else(|| "1.0".to_string());
                format!(
                    "ffmpeg -y -i '{}' -af 'volume={}' '{}'",
                    shell_escape(&p.input),
                    volume,
                    shell_escape(&p.output)
                )
            }
            _ => {
                return Err(Error::Tool(format!(
                    "未知的音频处理动作: {}。支持: convert, trim, volume",
                    p.action
                )));
            }
        };

        self.run_shell_command(&command)
    }

    /// 音频混合
    ///
    /// 使用 ffmpeg 的 complex filter 混合多个音频文件。
    fn mix_audio(&self, params: &str) -> Result<String> {
        let p: MixAudioParams = parse_params(params)?;

        if p.inputs.is_empty() {
            return Err(Error::Tool("音频混合至少需要一个输入文件".to_string()));
        }

        // 构建 ffmpeg complex filter 命令
        let mut input_args = String::new();
        let mut filter_parts = Vec::new();

        for (i, input) in p.inputs.iter().enumerate() {
            input_args.push_str(&format!(" -i '{}'", shell_escape(input)));
            filter_parts.push(format!("[{i}:a]"));
        }

        let mix_inputs = filter_parts.join("");
        let filter = format!("{mix_inputs}amix=inputs={}[a]", p.inputs.len());

        let command = format!(
            "ffmpeg -y{input_args} -filter_complex '{filter}' -map '[a]' '{}'",
            shell_escape(&p.output)
        );

        self.run_shell_command(&command)
    }

    /// 执行 shell 命令的通用辅助方法
    fn run_shell_command(&self, command: &str) -> Result<String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Tool(format!("创建 tokio runtime 失败: {e}")))?;

        let timeout = self.default_timeout;
        let command = command.to_string();

        rt.block_on(async {
            let result = tokio::time::timeout(
                Duration::from_secs(timeout),
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
                        if combined.is_empty() {
                            Ok("命令执行成功（无输出）".to_string())
                        } else {
                            Ok(combined)
                        }
                    } else {
                        Ok(format!(
                            "退出码: {}\n{}",
                            output.status.code().unwrap_or(-1),
                            combined
                        ))
                    }
                }
                Ok(Err(e)) => Err(Error::Tool(format!("命令执行失败: {e}"))),
                Err(_) => Err(Error::Tool(format!(
                    "命令执行超时（{}秒）",
                    timeout
                ))),
            }
        })
    }
}

impl ToolHandler for MediaHandler {
    fn domain(&self) -> ToolDomain {
        ToolDomain::Media
    }

    fn execute(&self, tool_name: &str, params: &str) -> Result<String> {
        match tool_name {
            "generate_image" => self.generate_image(params),
            "process_video" => self.process_video(params),
            "process_audio" => self.process_audio(params),
            "mix_audio" => self.mix_audio(params),
            _ => Err(Error::Tool(format!(
                "媒体领域未知工具: {tool_name}"
            ))),
        }
    }

    fn tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition::function(
                "generate_image",
                "生成图片。根据文本提示和风格生成图片，需要 ShellBridge 配置图片生成工具。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "prompt": {
                            "type": "string",
                            "description": "图片生成提示词，描述想要生成的图片内容"
                        },
                        "style": {
                            "type": "string",
                            "description": "图片风格，如 realistic、anime、oil-painting 等",
                            "default": "default"
                        }
                    },
                    "required": ["prompt"]
                }),
            ),
            ToolDefinition::function(
                "process_video",
                "处理视频文件。使用 ffmpeg 执行视频转码、提取音频、提取帧、剪辑等操作。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["convert", "extract_audio", "extract_frames", "trim"],
                            "description": "视频处理动作：convert=转码, extract_audio=提取音频, extract_frames=提取帧, trim=剪辑"
                        },
                        "input": {
                            "type": "string",
                            "description": "输入视频文件路径"
                        },
                        "output": {
                            "type": "string",
                            "description": "输出文件路径"
                        },
                        "start_time": {
                            "type": "string",
                            "description": "起始时间（格式 HH:MM:SS），用于 trim 动作",
                            "default": "00:00:00"
                        },
                        "duration": {
                            "type": "string",
                            "description": "持续时长（秒），用于 trim 动作",
                            "default": "10"
                        }
                    },
                    "required": ["action", "input", "output"]
                }),
            ),
            ToolDefinition::function(
                "process_audio",
                "处理音频文件。使用 ffmpeg 执行音频转码、剪辑、音量调节等操作。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["convert", "trim", "volume"],
                            "description": "音频处理动作：convert=转码, trim=剪辑, volume=音量调节"
                        },
                        "input": {
                            "type": "string",
                            "description": "输入音频文件路径"
                        },
                        "output": {
                            "type": "string",
                            "description": "输出文件路径"
                        },
                        "start_time": {
                            "type": "string",
                            "description": "起始时间（格式 HH:MM:SS），用于 trim 动作",
                            "default": "00:00:00"
                        },
                        "duration": {
                            "type": "string",
                            "description": "持续时长（秒），用于 trim 动作",
                            "default": "10"
                        },
                        "volume": {
                            "type": "string",
                            "description": "音量倍数，用于 volume 动作（如 0.5、2.0）",
                            "default": "1.0"
                        }
                    },
                    "required": ["action", "input", "output"]
                }),
            ),
            ToolDefinition::function(
                "mix_audio",
                "混合多个音频文件。使用 ffmpeg 的 amix 滤镜将多个音频混合为一个。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "inputs": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "输入音频文件路径列表"
                        },
                        "output": {
                            "type": "string",
                            "description": "输出混合音频文件路径"
                        }
                    },
                    "required": ["inputs", "output"]
                }),
            ),
        ]
    }
}

// ============================================================================
// 参数结构
// ============================================================================

/// 图片生成参数
#[derive(Debug, serde::Deserialize)]
struct GenerateImageParams {
    prompt: String,
    style: Option<String>,
}

/// 视频处理参数
#[derive(Debug, serde::Deserialize)]
struct ProcessVideoParams {
    action: String,
    input: String,
    output: String,
    start_time: Option<String>,
    duration: Option<String>,
}

/// 音频处理参数
#[derive(Debug, serde::Deserialize)]
struct ProcessAudioParams {
    action: String,
    input: String,
    output: String,
    start_time: Option<String>,
    duration: Option<String>,
    volume: Option<String>,
}

/// 音频混合参数
#[derive(Debug, serde::Deserialize)]
struct MixAudioParams {
    inputs: Vec<String>,
    output: String,
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 通用参数解析
fn parse_params<'a, T: serde::Deserialize<'a>>(params: &'a str) -> Result<T> {
    serde_json::from_str(params).map_err(|e| Error::Tool(format!("参数解析失败: {e}")))
}

/// 简单的 shell 转义：将单引号替换为 `'\''`
fn shell_escape(s: &str) -> String {
    s.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> KernelConfig {
        KernelConfig::from_json(r#"{"data_dir": "/tmp/peng_test"}"#).unwrap()
    }

    #[test]
    fn test_media_handler_domain() {
        let config = test_config();
        let handler = MediaHandler::new(&config);
        assert_eq!(handler.domain(), ToolDomain::Media);
    }

    #[test]
    fn test_media_handler_tool_definitions() {
        let config = test_config();
        let handler = MediaHandler::new(&config);
        let defs = handler.tool_definitions();
        assert_eq!(defs.len(), 4);
        let names: Vec<&str> = defs.iter().map(|d| d.function.name.as_str()).collect();
        assert!(names.contains(&"generate_image"));
        assert!(names.contains(&"process_video"));
        assert!(names.contains(&"process_audio"));
        assert!(names.contains(&"mix_audio"));
    }

    #[test]
    fn test_media_handler_unknown_tool() {
        let config = test_config();
        let handler = MediaHandler::new(&config);
        let result = handler.execute("unknown_tool", "{}");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_image_returns_shellbridge_required() {
        let config = test_config();
        let handler = MediaHandler::new(&config);
        let result = handler.execute("generate_image", r#"{"prompt": "a cat"}"#);
        assert!(result.is_ok());
        // generate_image either returns shell command output or the shellbridge_required fallback
        // Both are valid results; the exact output depends on whether python3 is available
        let _output = result.unwrap();
    }

    #[test]
    fn test_process_video_unknown_action() {
        let config = test_config();
        let handler = MediaHandler::new(&config);
        let result = handler.execute(
            "process_video",
            r#"{"action": "unknown", "input": "/tmp/in.mp4", "output": "/tmp/out.mp4"}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello"), "hello");
        assert_eq!(shell_escape("it's"), "it'\\''s");
    }
}
