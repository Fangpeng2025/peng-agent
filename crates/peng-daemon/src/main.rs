//! Peng Daemon - 独立的 AI Agent 服务
//!
//! 在 Ubuntu (proot) 环境中运行，通过 Unix Socket 提供 IPC 服务
//! 仅在 Unix 系统上可用

#[cfg(unix)]
mod ipc_unix;
#[cfg(unix)]
mod protocol;

#[cfg(unix)]
use anyhow::{Context, Result};
#[cfg(unix)]
use log::info;
#[cfg(unix)]
use std::env;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::sync::atomic::AtomicBool;
#[cfg(unix)]
use std::sync::Arc;

#[cfg(unix)]
use peng_core::{AgentLoop, KernelConfig};

#[cfg(unix)]
fn main() -> Result<()> {
    use ipc_unix::IpcServer;
    use std::sync::Mutex;
    
    // 初始化日志
    env_logger::Builder::from_env("PENG_LOG")
        .filter_level(log::LevelFilter::Info)
        .init();
    
    info!("🚀 Peng Daemon starting...");
    
    // 解析命令行参数
    let socket_path = env::var("PENG_SOCKET")
        .unwrap_or_else(|_| "/data/data/com.peng.agent/files/peng.sock".to_string());
    let data_dir = env::var("PENG_DATA_DIR")
        .unwrap_or_else(|_| "/data/data/com.peng.agent/files/data".to_string());
    
    info!("Socket path: {}", socket_path);
    info!("Data directory: {}", data_dir);
    
    // 创建数据目录
    std::fs::create_dir_all(&data_dir)
        .context("Failed to create data directory")?;
    
    // 加载配置
    let config = load_config(&data_dir)?;
    
    // 初始化 Agent
    info!("Initializing Agent...");
    let agent = AgentLoop::new(config)
        .context("Failed to initialize agent")?;
    
    // 清理旧的 Socket
    if Path::new(&socket_path).exists() {
        std::fs::remove_file(&socket_path)
            .context("Failed to remove old socket")?;
    }
    
    // 运行标志
    let running = Arc::new(AtomicBool::new(true));
    
    // 启动 IPC 服务
    info!("Starting IPC server...");
    let agent_arc = Arc::new(Mutex::new(Some(agent)));
    let server = IpcServer::new(&socket_path, agent_arc, running.clone())
        .context("Failed to create IPC server")?;
    
    info!("✅ Peng Daemon ready, listening on {}", socket_path);
    
    // 运行服务
    server.run()?;
    
    info!("Peng Daemon stopped");
    Ok(())
}

#[cfg(unix)]
fn load_config(data_dir: &str) -> Result<KernelConfig> {
    let config_path = format!("{}/config.json", data_dir);
    
    // 尝试从文件加载
    if Path::new(&config_path).exists() {
        let content = std::fs::read_to_string(&config_path)?;
        return KernelConfig::from_json(&content)
            .map_err(|e| anyhow::anyhow!("Config parse error: {}", e));
    }
    
    // 从环境变量加载
    KernelConfig::from_env(data_dir)
        .map_err(|e| anyhow::anyhow!("Config load error: {}", e))
}

// 非 Unix 系统的占位 main
#[cfg(not(unix))]
fn main() {
    println!("peng-daemon only runs on Unix systems (Linux/Android)");
    println!("This binary is designed to run inside Ubuntu proot environment");
}