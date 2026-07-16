//! 文件领域工具处理器
//!
//! 提供文件和目录操作工具（Rust 原生实现，不依赖 shell）：
//! - `read_file` — 读取文件内容
//! - `write_file` — 写入文件
//! - `list_directory` — 列出目录内容
//! - `create_directory` — 创建目录（含父目录）
//! - `delete_file` — 删除文件或目录
//! - `file_info` — 获取文件元信息
//!
//! 所有路径均相对于 `config.data_dir`，除非路径是绝对路径。

use std::path::{Path, PathBuf};

use crate::config::KernelConfig;
use crate::llm::ToolDefinition;
use crate::types::{Error, Result, ToolDomain};

use super::router::ToolHandler;

// ============================================================================
// FileHandler
// ============================================================================

/// 文件领域工具处理器
///
/// 使用 Rust 标准库进行文件操作，不依赖外部命令。
/// 所有路径自动解析为相对于 `data_dir` 的路径（绝对路径除外）。
pub struct FileHandler {
    /// 数据根目录，相对路径基于此解析
    data_dir: PathBuf,
}

impl FileHandler {
    /// 创建文件领域处理器
    pub fn new(config: &KernelConfig) -> Self {
        Self {
            data_dir: PathBuf::from(&config.data_dir),
        }
    }

    /// 将路径解析为绝对路径
    ///
    /// 如果路径已经是绝对路径，直接使用；否则相对于 `data_dir` 解析。
    fn resolve_path(&self, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.data_dir.join(p)
        }
    }

    /// 读取文件内容
    fn read_file(&self, params: &str) -> Result<String> {
        let p: FilePathParams = parse_params(params)?;
        let path = self.resolve_path(&p.path);

        let content = std::fs::read_to_string(&path).map_err(|e| {
            Error::Io(format!("读取文件失败 ({}): {e}", path.display()))
        })?;

        Ok(content)
    }

    /// 写入文件
    fn write_file(&self, params: &str) -> Result<String> {
        let p: WriteFileParams = parse_params(params)?;
        let path = self.resolve_path(&p.path);

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Io(format!("创建父目录失败 ({}): {e}", parent.display()))
            })?;
        }

        std::fs::write(&path, &p.content).map_err(|e| {
            Error::Io(format!("写入文件失败 ({}): {e}", path.display()))
        })?;

        Ok(format!("文件写入成功: {}", path.display()))
    }

    /// 列出目录内容
    fn list_directory(&self, params: &str) -> Result<String> {
        let p: FilePathParams = parse_params(params)?;
        let path = self.resolve_path(&p.path);

        let entries: Vec<serde_json::Value> = std::fs::read_dir(&path)
            .map_err(|e| Error::Io(format!("读取目录失败 ({}): {e}", path.display())))?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().to_string_lossy().to_string();
                let file_type = entry
                    .file_type()
                    .ok()
                    .map(|ft| {
                        if ft.is_dir() {
                            "directory"
                        } else if ft.is_symlink() {
                            "symlink"
                        } else {
                            "file"
                        }
                    })
                    .unwrap_or("unknown");

                let size = entry.metadata().ok().map(|m| m.len()).unwrap_or(0);

                Some(serde_json::json!({
                    "name": name,
                    "type": file_type,
                    "size": size,
                }))
            })
            .collect();

        Ok(serde_json::to_string_pretty(&entries)
            .unwrap_or_else(|e| format!("{{\"error\": \"序列化失败: {e}\"}}")))
    }

    /// 创建目录（含父目录）
    fn create_directory(&self, params: &str) -> Result<String> {
        let p: FilePathParams = parse_params(params)?;
        let path = self.resolve_path(&p.path);

        std::fs::create_dir_all(&path).map_err(|e| {
            Error::Io(format!("创建目录失败 ({}): {e}", path.display()))
        })?;

        Ok(format!("目录创建成功: {}", path.display()))
    }

    /// 删除文件或目录
    fn delete_file(&self, params: &str) -> Result<String> {
        let p: FilePathParams = parse_params(params)?;
        let path = self.resolve_path(&p.path);

        if path.is_dir() {
            std::fs::remove_dir_all(&path).map_err(|e| {
                Error::Io(format!("删除目录失败 ({}): {e}", path.display()))
            })?;
            Ok(format!("目录删除成功: {}", path.display()))
        } else {
            std::fs::remove_file(&path).map_err(|e| {
                Error::Io(format!("删除文件失败 ({}): {e}", path.display()))
            })?;
            Ok(format!("文件删除成功: {}", path.display()))
        }
    }

    /// 获取文件元信息
    fn file_info(&self, params: &str) -> Result<String> {
        let p: FilePathParams = parse_params(params)?;
        let path = self.resolve_path(&p.path);

        let metadata = std::fs::metadata(&path).map_err(|e| {
            Error::Io(format!("获取文件信息失败 ({}): {e}", path.display()))
        })?;

        let file_type = if metadata.is_dir() {
            "directory"
        } else if metadata.is_symlink() {
            "symlink"
        } else {
            "file"
        };

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            format!("{:o}", metadata.permissions().mode())
        };
        #[cfg(not(unix))]
        let permissions = {
            if metadata.permissions().readonly() {
                "readonly".to_string()
            } else {
                "readwrite".to_string()
            }
        };

        let info = serde_json::json!({
            "path": path.to_string_lossy().to_string(),
            "type": file_type,
            "size": metadata.len(),
            "modified_unix": modified,
            "permissions": permissions,
            "readonly": metadata.permissions().readonly(),
        });

        Ok(serde_json::to_string_pretty(&info)
            .unwrap_or_else(|e| format!("{{\"error\": \"序列化失败: {e}\"}}")))
    }
}

impl ToolHandler for FileHandler {
    fn domain(&self) -> ToolDomain {
        ToolDomain::File
    }

    fn execute(&self, tool_name: &str, params: &str) -> Result<String> {
        match tool_name {
            "read_file" => self.read_file(params),
            "write_file" => self.write_file(params),
            "list_directory" => self.list_directory(params),
            "create_directory" => self.create_directory(params),
            "delete_file" => self.delete_file(params),
            "file_info" => self.file_info(params),
            _ => Err(Error::Tool(format!(
                "文件领域未知工具: {tool_name}"
            ))),
        }
    }

    fn tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition::function(
                "read_file",
                "读取文件内容。路径相对于数据目录，绝对路径直接使用。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "文件路径（相对路径基于数据目录，或绝对路径）"
                        }
                    },
                    "required": ["path"]
                }),
            ),
            ToolDefinition::function(
                "write_file",
                "写入文件。自动创建父目录。路径相对于数据目录，绝对路径直接使用。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "文件路径（相对路径基于数据目录，或绝对路径）"
                        },
                        "content": {
                            "type": "string",
                            "description": "要写入的文件内容"
                        }
                    },
                    "required": ["path", "content"]
                }),
            ),
            ToolDefinition::function(
                "list_directory",
                "列出目录内容。返回目录中所有文件和子目录的名称、类型和大小。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "目录路径（相对路径基于数据目录，或绝对路径）",
                            "default": "."
                        }
                    },
                    "required": ["path"]
                }),
            ),
            ToolDefinition::function(
                "create_directory",
                "创建目录。自动创建所有不存在的父目录。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "要创建的目录路径"
                        }
                    },
                    "required": ["path"]
                }),
            ),
            ToolDefinition::function(
                "delete_file",
                "删除文件或目录。如果是目录，将递归删除所有内容。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "要删除的文件或目录路径"
                        }
                    },
                    "required": ["path"]
                }),
            ),
            ToolDefinition::function(
                "file_info",
                "获取文件或目录的元信息，包括大小、修改时间、权限等。",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "文件或目录路径"
                        }
                    },
                    "required": ["path"]
                }),
            ),
        ]
    }
}

// ============================================================================
// 参数结构
// ============================================================================

/// 文件路径参数
#[derive(Debug, serde::Deserialize)]
struct FilePathParams {
    path: String,
}

/// 写入文件参数
#[derive(Debug, serde::Deserialize)]
struct WriteFileParams {
    path: String,
    content: String,
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
        KernelConfig::from_json(r#"{"data_dir": "/tmp/peng_test_file"}"#).unwrap()
    }

    #[test]
    fn test_file_handler_domain() {
        let config = test_config();
        let handler = FileHandler::new(&config);
        assert_eq!(handler.domain(), ToolDomain::File);
    }

    #[test]
    fn test_file_handler_tool_definitions() {
        let config = test_config();
        let handler = FileHandler::new(&config);
        let defs = handler.tool_definitions();
        assert_eq!(defs.len(), 6);
        let names: Vec<&str> = defs.iter().map(|d| d.function.name.as_str()).collect();
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"list_directory"));
        assert!(names.contains(&"create_directory"));
        assert!(names.contains(&"delete_file"));
        assert!(names.contains(&"file_info"));
    }

    #[test]
    fn test_resolve_path_relative() {
        let config = test_config();
        let handler = FileHandler::new(&config);
        let resolved = handler.resolve_path("test.txt");
        assert_eq!(resolved, PathBuf::from("/tmp/peng_test_file/test.txt"));
    }

    #[test]
    fn test_resolve_path_absolute() {
        let config = test_config();
        let handler = FileHandler::new(&config);
        let resolved = handler.resolve_path("/absolute/path.txt");
        assert_eq!(resolved, PathBuf::from("/absolute/path.txt"));
    }

    #[test]
    fn test_file_handler_unknown_tool() {
        let config = test_config();
        let handler = FileHandler::new(&config);
        let result = handler.execute("unknown_tool", "{}");
        assert!(result.is_err());
    }

    #[test]
    fn test_write_and_read_file() {
        let config = test_config();
        let handler = FileHandler::new(&config);

        // 先创建目录
        let _ = handler.execute("create_directory", r#"{"path": "/tmp/peng_test_file_rw"}"#);

        // 写入文件
        let write_result = handler.execute(
            "write_file",
            r#"{"path": "/tmp/peng_test_file_rw/test.txt", "content": "hello world"}"#,
        );
        assert!(write_result.is_ok());

        // 读取文件
        let read_result = handler.execute(
            "read_file",
            r#"{"path": "/tmp/peng_test_file_rw/test.txt"}"#,
        );
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), "hello world");

        // 清理
        let _ = handler.execute("delete_file", r#"{"path": "/tmp/peng_test_file_rw"}"#);
    }

    #[test]
    fn test_file_info() {
        let config = test_config();
        let handler = FileHandler::new(&config);

        // 先创建文件
        let _ = handler.execute("create_directory", r#"{"path": "/tmp/peng_test_file_info"}"#);
        let _ = handler.execute(
            "write_file",
            r#"{"path": "/tmp/peng_test_file_info/info_test.txt", "content": "test"}"#,
        );

        let result = handler.execute(
            "file_info",
            r#"{"path": "/tmp/peng_test_file_info/info_test.txt"}"#,
        );
        assert!(result.is_ok());
        let info: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(info["type"], "file");
        assert_eq!(info["size"], 4);

        // 清理
        let _ = handler.execute("delete_file", r#"{"path": "/tmp/peng_test_file_info"}"#);
    }
}
