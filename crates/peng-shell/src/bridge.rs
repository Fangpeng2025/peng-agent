//! peng-shell: ShellBridge — Termux 命令执行桥
//!
//! ShellBridge 提供在 Termux 环境中执行 shell/python/node 命令的能力，
//! 支持超时控制、环境变量配置和引导状态检查。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use peng_core::types::{Error, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

// ============================================================================
// ShellResult — 命令执行结果
// ============================================================================

/// 命令执行结果
///
/// 包含命令的退出码、标准输出、标准错误以及是否超时的标记。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellResult {
    /// 命令退出码（0 表示成功）
    pub exit_code: i32,
    /// 标准输出内容
    pub stdout: String,
    /// 标准错误内容
    pub stderr: String,
    /// 是否因超时而被终止
    pub timed_out: bool,
}

impl ShellResult {
    /// 判断命令是否执行成功
    pub fn is_success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }

    /// 合并标准输出和标准错误
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

// ============================================================================
// BootstrapStatus — 引导状态
// ============================================================================

/// Termux 环境引导状态
///
/// 检测 Termux 环境中各关键组件的可用性。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapStatus {
    /// sh 是否可用
    pub sh_available: bool,
    /// python3 是否可用
    pub python_available: bool,
    /// node 是否可用
    pub node_available: bool,
    /// ffmpeg 是否可用
    pub ffmpeg_available: bool,
    /// Termux prefix 路径是否有效
    pub prefix_valid: bool,
}

impl BootstrapStatus {
    /// 所有必需组件是否均已就绪
    ///
    /// 至少需要 sh 可用且 prefix 有效。
    pub fn is_ready(&self) -> bool {
        self.sh_available && self.prefix_valid
    }

    /// 获取缺失组件的描述列表
    pub fn missing_components(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.sh_available {
            missing.push("sh");
        }
        if !self.python_available {
            missing.push("python3");
        }
        if !self.node_available {
            missing.push("node");
        }
        if !self.ffmpeg_available {
            missing.push("ffmpeg");
        }
        if !self.prefix_valid {
            missing.push("prefix");
        }
        missing
    }
}

impl std::fmt::Display for BootstrapStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BootstrapStatus {{ sh: {}, python: {}, node: {}, ffmpeg: {}, prefix: {} }}",
            self.sh_available,
            self.python_available,
            self.node_available,
            self.ffmpeg_available,
            self.prefix_valid
        )
    }
}

// ============================================================================
// ShellBridge — Termux 命令执行桥
// ============================================================================

/// Termux ShellBridge
///
/// 在 Termux 环境中执行 shell/python/node 命令，支持：
/// - 环境变量自动配置（PATH, HOME, LD_LIBRARY_PATH, TERMUX_PREFIX）
/// - 超时控制
/// - 引导状态检查
pub struct ShellBridge {
    /// Termux prefix 路径（如 /data/data/com.peng.agent/files/usr）
    prefix_path: String,
    /// HOME 目录路径
    home_path: String,
    /// bin 目录路径
    bin_path: String,
    /// 是否已初始化
    initialized: AtomicBool,
}

impl ShellBridge {
    /// 创建新的 ShellBridge 实例
    ///
    /// # 参数
    /// - `prefix_path`: Termux prefix 路径，例如 `/data/data/com.peng.agent/files/usr`
    ///
    /// # 返回
    /// 未初始化的 ShellBridge 实例
    pub fn new(prefix_path: &str) -> Self {
        let home_path = if prefix_path.ends_with("/usr") {
            // 典型 Termux 路径: prefix = .../usr, home = .../home
            prefix_path.trim_end_matches("/usr").to_string() + "/home"
        } else {
            prefix_path.to_string() + "/home"
        };
        let bin_path = prefix_path.to_string() + "/bin";

        Self {
            prefix_path: prefix_path.to_string(),
            home_path,
            bin_path,
            initialized: AtomicBool::new(false),
        }
    }

    /// 初始化 ShellBridge，验证 Termux 环境
    ///
    /// 检查 sh/python3/node 是否存在，并将初始化状态标记为已完成。
    pub fn initialize(&self) -> Result<()> {
        let status = self.bootstrap_check();

        if !status.prefix_valid {
            return Err(Error::Other(format!(
                "Termux prefix 路径无效: {}",
                self.prefix_path
            )));
        }

        if !status.sh_available {
            return Err(Error::Other(format!(
                "sh 不存在于: {}",
                self.bin_path
            )));
        }

        if !status.python_available {
            log::warn!("python3 在 Termux 环境中不可用，部分功能受限");
        }

        if !status.node_available {
            log::warn!("node 在 Termux 环境中不可用，部分功能受限");
        }

        self.initialized.store(true, Ordering::SeqCst);
        log::info!("ShellBridge 初始化完成: prefix={}", self.prefix_path);
        Ok(())
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// 执行 shell 命令（核心方法）
    ///
    /// 设置 Termux 环境变量后，通过 `sh -c <command>` 执行命令，
    /// 并在指定超时后强制终止进程。
    ///
    /// # 参数
    /// - `command`: 要执行的 shell 命令
    /// - `timeout_secs`: 超时秒数
    ///
    /// # 返回
    /// 命令执行结果 [`ShellResult`]
    pub async fn execute(&self, command: &str, timeout_secs: u64) -> Result<ShellResult> {
        let output = Command::new(format!("{}/sh", self.bin_path))
            .arg("-c")
            .arg(command)
            .env("PATH", self.build_path_env())
            .env("HOME", &self.home_path)
            .env("LD_LIBRARY_PATH", format!("{}/lib", self.prefix_path))
            .env("TERMUX_PREFIX", &self.prefix_path)
            .env("LANG", "en_US.UTF-8")
            .output();

        let result = match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            output,
        )
        .await
        {
            Ok(Ok(output)) => ShellResult {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                timed_out: false,
            },
            Ok(Err(e)) => {
                return Err(Error::Io(format!("命令执行失败: {e}")));
            }
            Err(_) => ShellResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("命令执行超时（{}秒）", timeout_secs),
                timed_out: true,
            },
        };

        Ok(result)
    }

    /// 执行 Python 代码
    ///
    /// 通过 `python3 -c <code>` 执行 Python 代码片段。
    ///
    /// # 参数
    /// - `code`: Python 代码字符串
    ///
    /// # 返回
    /// 命令执行结果 [`ShellResult`]
    pub async fn execute_python(&self, code: &str) -> Result<ShellResult> {
        let python_path = format!("{}/python3", self.bin_path);
        if !Path::new(&python_path).exists() {
            return Err(Error::Other("python3 在 Termux 环境中不可用".to_string()));
        }

        let output = Command::new(&python_path)
            .arg("-c")
            .arg(code)
            .env("PATH", self.build_path_env())
            .env("HOME", &self.home_path)
            .env("LD_LIBRARY_PATH", format!("{}/lib", self.prefix_path))
            .env("TERMUX_PREFIX", &self.prefix_path)
            .env("LANG", "en_US.UTF-8")
            .output()
            .await
            .map_err(|e| Error::Io(format!("python3 执行失败: {e}")))?;

        Ok(ShellResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            timed_out: false,
        })
    }

    /// 执行 Node.js 代码
    ///
    /// 通过 `node -e <code>` 执行 JavaScript 代码片段。
    ///
    /// # 参数
    /// - `code`: JavaScript 代码字符串
    ///
    /// # 返回
    /// 命令执行结果 [`ShellResult`]
    pub async fn execute_node(&self, code: &str) -> Result<ShellResult> {
        let node_path = format!("{}/node", self.bin_path);
        if !Path::new(&node_path).exists() {
            return Err(Error::Other("node 在 Termux 环境中不可用".to_string()));
        }

        let output = Command::new(&node_path)
            .arg("-e")
            .arg(code)
            .env("PATH", self.build_path_env())
            .env("HOME", &self.home_path)
            .env("LD_LIBRARY_PATH", format!("{}/lib", self.prefix_path))
            .env("TERMUX_PREFIX", &self.prefix_path)
            .env("LANG", "en_US.UTF-8")
            .output()
            .await
            .map_err(|e| Error::Io(format!("node 执行失败: {e}")))?;

        Ok(ShellResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            timed_out: false,
        })
    }

    /// 执行脚本文件
    ///
    /// 通过 `sh <script_path> <args...>` 执行脚本文件。
    ///
    /// # 参数
    /// - `script_path`: 脚本文件路径
    /// - `args`: 传递给脚本的参数列表
    ///
    /// # 返回
    /// 命令执行结果 [`ShellResult`]
    pub async fn execute_script(&self, script_path: &str, args: &[&str]) -> Result<ShellResult> {
        if !Path::new(script_path).exists() {
            return Err(Error::Io(format!("脚本文件不存在: {script_path}")));
        }

        let mut cmd = Command::new(format!("{}/sh", self.bin_path));
        cmd.arg(script_path)
            .args(args)
            .env("PATH", self.build_path_env())
            .env("HOME", &self.home_path)
            .env("LD_LIBRARY_PATH", format!("{}/lib", self.prefix_path))
            .env("TERMUX_PREFIX", &self.prefix_path)
            .env("LANG", "en_US.UTF-8");

        let output = cmd
            .output()
            .await
            .map_err(|e| Error::Io(format!("脚本执行失败: {e}")))?;

        Ok(ShellResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            timed_out: false,
        })
    }

    /// 查找命令路径
    ///
    /// 在 Termux 的 PATH 中查找指定命令的完整路径。
    ///
    /// # 参数
    /// - `command`: 要查找的命令名称
    ///
    /// # 返回
    /// 命令的完整路径，未找到则返回错误
    pub async fn which(&self, command: &str) -> Result<String> {
        // 首先检查 bin 目录下是否直接存在
        let direct_path = format!("{}/{}", self.bin_path, command);
        if Path::new(&direct_path).exists() {
            return Ok(direct_path);
        }

        // 尝试 which 命令
        let result = self
            .execute(&format!("which {command}"), 10)
            .await?;

        if result.is_success() {
            let path = result.stdout.trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }

        Err(Error::Other(format!("命令未找到: {command}")))
    }

    /// 检查 Termux 环境引导状态
    ///
    /// 检测 sh/python3/node/ffmpeg 是否可用，以及 prefix 路径是否有效。
    pub fn bootstrap_check(&self) -> BootstrapStatus {
        let prefix_path = Path::new(&self.prefix_path);

        let prefix_valid = prefix_path.exists() && prefix_path.is_dir();

        let sh_available = Path::new(&format!("{}/sh", self.bin_path)).exists();
        let python_available = Path::new(&format!("{}/python3", self.bin_path)).exists();
        let node_available = Path::new(&format!("{}/node", self.bin_path)).exists();
        let ffmpeg_available = Path::new(&format!("{}/ffmpeg", self.bin_path)).exists();

        BootstrapStatus {
            sh_available,
            python_available,
            node_available,
            ffmpeg_available,
            prefix_valid,
        }
    }

    /// 获取 prefix 路径
    pub fn prefix_path(&self) -> &str {
        &self.prefix_path
    }

    /// 获取 HOME 路径
    pub fn home_path(&self) -> &str {
        &self.home_path
    }

    /// 获取 bin 路径
    pub fn bin_path(&self) -> &str {
        &self.bin_path
    }

    // ---- 内部辅助方法 ----

    /// 构建 PATH 环境变量值
    fn build_path_env(&self) -> String {
        format!(
            "{bin}:{bin}/applets:/system/bin:/system/xbin",
            bin = self.bin_path
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_result_success() {
        let result = ShellResult {
            exit_code: 0,
            stdout: "hello".to_string(),
            stderr: String::new(),
            timed_out: false,
        };
        assert!(result.is_success());
        assert_eq!(result.combined_output(), "hello");
    }

    #[test]
    fn test_shell_result_failure() {
        let result = ShellResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
            timed_out: false,
        };
        assert!(!result.is_success());
        assert_eq!(result.combined_output(), "error");
    }

    #[test]
    fn test_shell_result_timeout() {
        let result = ShellResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: "timeout".to_string(),
            timed_out: true,
        };
        assert!(!result.is_success());
    }

    #[test]
    fn test_combined_output_both() {
        let result = ShellResult {
            exit_code: 0,
            stdout: "out".to_string(),
            stderr: "err".to_string(),
            timed_out: false,
        };
        assert_eq!(result.combined_output(), "out\nerr");
    }

    #[test]
    fn test_bootstrap_status_ready() {
        let status = BootstrapStatus {
            sh_available: true,
            python_available: true,
            node_available: false,
            ffmpeg_available: false,
            prefix_valid: true,
        };
        assert!(status.is_ready());
        assert!(!status.missing_components().contains(&"sh"));
    }

    #[test]
    fn test_bootstrap_status_not_ready() {
        let status = BootstrapStatus {
            sh_available: false,
            python_available: false,
            node_available: false,
            ffmpeg_available: false,
            prefix_valid: false,
        };
        assert!(!status.is_ready());
        assert_eq!(status.missing_components().len(), 5);
    }

    #[test]
    fn test_shell_bridge_new() {
        let bridge = ShellBridge::new("/data/data/com.peng.agent/files/usr");
        assert_eq!(bridge.prefix_path(), "/data/data/com.peng.agent/files/usr");
        assert_eq!(bridge.home_path(), "/data/data/com.peng.agent/files/home");
        assert_eq!(bridge.bin_path(), "/data/data/com.peng.agent/files/usr/bin");
        assert!(!bridge.is_initialized());
    }

    #[test]
    fn test_shell_bridge_new_no_usr_suffix() {
        let bridge = ShellBridge::new("/opt/termux");
        assert_eq!(bridge.prefix_path(), "/opt/termux");
        assert_eq!(bridge.home_path(), "/opt/termux/home");
        assert_eq!(bridge.bin_path(), "/opt/termux/bin");
    }

    #[test]
    fn test_build_path_env() {
        let bridge = ShellBridge::new("/data/data/com.peng.agent/files/usr");
        let path_env = bridge.build_path_env();
        assert!(path_env.contains("/data/data/com.peng.agent/files/usr/bin"));
        assert!(path_env.contains("/applets"));
    }
}
