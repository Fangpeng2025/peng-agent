//! peng-shell: BootstrapManager — Termux 环境引导与验证
//!
//! BootstrapManager 负责验证 Termux 环境的完整性，确保必要的目录结构、
//! 符号链接和共享库均已就绪。

use std::fs;
use std::path::Path;

use peng_core::types::{Error, Result};

use crate::bridge::BootstrapStatus;

// ============================================================================
// BootstrapManager — Termux 环境引导管理器
// ============================================================================

/// Termux 环境引导管理器
///
/// 负责验证 Termux 运行环境的完整性，包括：
/// - 目录结构验证与创建
/// - PATH 配置与符号链接
/// - 共享库完整性检查
pub struct BootstrapManager {
    /// Termux prefix 路径
    prefix_path: String,
    /// 解压后的 assets 路径
    assets_path: String,
}

impl BootstrapManager {
    /// 创建新的 BootstrapManager 实例
    ///
    /// # 参数
    /// - `prefix_path`: Termux prefix 路径
    /// - `assets_path`: 解压后的 assets 路径
    pub fn new(prefix_path: &str, assets_path: &str) -> Self {
        Self {
            prefix_path: prefix_path.to_string(),
            assets_path: assets_path.to_string(),
        }
    }

    /// 验证 Termux 环境状态
    ///
    /// 检查各关键组件和路径是否可用。
    ///
    /// # 返回
    /// 引导状态 [`BootstrapStatus`]
    pub fn verify_environment(&self) -> Result<BootstrapStatus> {
        let prefix_path = Path::new(&self.prefix_path);

        let prefix_valid = prefix_path.exists() && prefix_path.is_dir();

        let bin_path = prefix_path.join("bin");
        let sh_available = bin_path.join("sh").exists();
        let python_available = bin_path.join("python3").exists();
        let node_available = bin_path.join("node").exists();
        let ffmpeg_available = bin_path.join("ffmpeg").exists();

        let status = BootstrapStatus {
            sh_available,
            python_available,
            node_available,
            ffmpeg_available,
            prefix_valid,
        };

        log::info!("Termux 环境验证: {}", status);

        if !status.is_ready() {
            log::warn!(
                "Termux 环境不完整，缺失组件: {:?}",
                status.missing_components()
            );
        }

        Ok(status)
    }

    /// 确保必要的目录结构存在
    ///
    /// 创建以下目录（如不存在）：
    /// - `{prefix}/bin`
    /// - `{prefix}/lib`
    /// - `{prefix}/tmp`
    /// - `{prefix}/../home` (HOME 目录)
    pub fn ensure_directories(&self) -> Result<()> {
        let prefix = Path::new(&self.prefix_path);

        let dirs = [
            prefix.join("bin"),
            prefix.join("lib"),
            prefix.join("tmp"),
            prefix.join("etc"),
        ];

        for dir in &dirs {
            if !dir.exists() {
                fs::create_dir_all(dir).map_err(|e| {
                    Error::Io(format!("创建目录失败 {}: {e}", dir.display()))
                })?;
                log::info!("创建目录: {}", dir.display());
            }
        }

        // HOME 目录: prefix 去掉 /usr 后的 /home
        let home_dir = if self.prefix_path.ends_with("/usr") || self.prefix_path.ends_with("\\usr") {
            prefix
                .parent()
                .map(|p| p.join("home"))
                .unwrap_or_else(|| prefix.join("home"))
        } else {
            prefix.join("home")
        };

        if !home_dir.exists() {
            fs::create_dir_all(&home_dir).map_err(|e| {
                Error::Io(format!("创建 HOME 目录失败 {}: {e}", home_dir.display()))
            })?;
            log::info!("创建 HOME 目录: {}", home_dir.display());
        }

        // tmp 目录需要设置权限（在 Android 上可能受限）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let tmp_dir = prefix.join("tmp");
            let _ = fs::set_permissions(&tmp_dir, fs::Permissions::from_mode(0o777));
        }

        Ok(())
    }

    /// 确保 PATH 所需的符号链接存在
    ///
    /// 在 bin 目录下创建必要的符号链接，确保常用命令可被发现。
    /// 如果 assets 目录中存在可执行文件，创建指向它们的符号链接。
    pub fn setup_path(&self) -> Result<()> {
        let bin_dir = Path::new(&self.prefix_path).join("bin");

        if !bin_dir.exists() {
            fs::create_dir_all(&bin_dir).map_err(|e| {
                Error::Io(format!("创建 bin 目录失败 {}: {e}", bin_dir.display()))
            })?;
        }

        // 确保 applets 目录存在
        let applets_dir = bin_dir.join("applets");
        if !applets_dir.exists() {
            fs::create_dir_all(&applets_dir).map_err(|e| {
                Error::Io(format!(
                    "创建 applets 目录失败 {}: {e}",
                    applets_dir.display()
                ))
            })?;
        }

        // 从 assets 目录复制/链接可执行文件到 bin 目录
        let assets_bin = Path::new(&self.assets_path).join("bin");
        if assets_bin.exists() && assets_bin.is_dir() {
            if let Ok(entries) = fs::read_dir(&assets_bin) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let target = bin_dir.join(&name);

                    // 仅在目标不存在时创建符号链接
                    if !target.exists() {
                        let source = entry.path();
                        #[cfg(unix)]
                        {
                            if let Err(e) = std::os::unix::fs::symlink(&source, &target) {
                                log::warn!(
                                    "创建符号链接失败 {} -> {}: {e}",
                                    target.display(),
                                    source.display()
                                );
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            // 非 Unix 系统（如 Windows 开发环境），使用复制代替
                            if source.is_file() {
                                if let Err(e) = fs::copy(&source, &target) {
                                    log::warn!(
                                        "复制文件失败 {} -> {}: {e}",
                                        source.display(),
                                        target.display()
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        log::info!("PATH 设置完成: {}", bin_dir.display());
        Ok(())
    }

    /// 验证共享库文件
    ///
    /// 检查指定目录下的 .so 文件是否都存在且可读。
    ///
    /// # 参数
    /// - `lib_dir`: 共享库目录路径
    ///
    /// # 返回
    /// 验证通过返回 Ok(())，否则返回错误
    pub fn install_shared_libs(&self, lib_dir: &str) -> Result<()> {
        let lib_path = Path::new(lib_dir);

        if !lib_path.exists() {
            return Err(Error::Io(format!("共享库目录不存在: {lib_dir}")));
        }

        let mut found_count = 0usize;
        let mut error_count = 0usize;

        if let Ok(entries) = fs::read_dir(lib_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "so" {
                        found_count += 1;
                        // 尝试读取文件元数据以验证可访问性
                        if let Err(e) = fs::metadata(&path) {
                            log::warn!("共享库不可访问 {}: {e}", path.display());
                            error_count += 1;
                        }
                    }
                }
            }
        }

        if error_count > 0 {
            return Err(Error::Other(format!(
                "共享库验证失败: {error_count}/{found_count} 个文件不可访问"
            )));
        }

        log::info!("共享库验证通过: {found_count} 个 .so 文件");
        Ok(())
    }

    /// 执行完整引导流程
    ///
    /// 依次执行：目录创建 → PATH 设置 → 共享库验证 → 环境验证
    ///
    /// # 返回
    /// 引导完成后的环境状态 [`BootstrapStatus`]
    pub fn bootstrap(&self) -> Result<BootstrapStatus> {
        log::info!("开始 Termux 环境引导: prefix={}", self.prefix_path);

        // 1. 创建必要目录
        self.ensure_directories()?;

        // 2. 设置 PATH 和符号链接
        self.setup_path()?;

        // 3. 验证共享库
        let lib_dir = format!("{}/lib", self.prefix_path);
        if Path::new(&lib_dir).exists() {
            self.install_shared_libs(&lib_dir)?;
        } else {
            log::warn!("共享库目录不存在，跳过验证: {lib_dir}");
        }

        // 4. 验证环境
        let status = self.verify_environment()?;

        if status.is_ready() {
            log::info!("Termux 环境引导完成");
        } else {
            log::warn!(
                "Termux 环境引导完成，但存在缺失组件: {:?}",
                status.missing_components()
            );
        }

        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_manager_new() {
        let mgr = BootstrapManager::new("/data/data/com.peng.agent/files/usr", "/assets");
        assert_eq!(mgr.prefix_path, "/data/data/com.peng.agent/files/usr");
        assert_eq!(mgr.assets_path, "/assets");
    }

    #[test]
    fn test_verify_environment_nonexistent() {
        let mgr = BootstrapManager::new("/nonexistent/path/usr", "/nonexistent/assets");
        let status = mgr.verify_environment().unwrap();
        assert!(!status.prefix_valid);
        assert!(!status.sh_available);
        assert!(!status.is_ready());
    }

    #[test]
    fn test_ensure_directories_creates_dirs() {
        let tmp = std::env::temp_dir().join("peng_shell_test_bootstrap");
        let prefix = tmp.join("usr");
        let _ = fs::remove_dir_all(&tmp);

        let mgr = BootstrapManager::new(prefix.to_str().unwrap(), "/nonexistent/assets");
        mgr.ensure_directories().unwrap();

        assert!(prefix.join("bin").exists());
        assert!(prefix.join("lib").exists());
        assert!(prefix.join("tmp").exists());
        assert!(prefix.join("etc").exists());
        assert!(tmp.join("home").exists());

        // 清理
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_install_shared_libs_missing_dir() {
        let mgr = BootstrapManager::new("/tmp/test", "/tmp/test");
        let result = mgr.install_shared_libs("/nonexistent/lib");
        assert!(result.is_err());
    }
}
