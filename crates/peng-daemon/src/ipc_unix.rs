//! IPC Server - Unix Socket 服务器

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use log::{info, error, warn};
use tokio::runtime::Runtime;

use peng_core::{AgentLoop, ChatMessage, StreamCallback, KernelStatus};
use crate::protocol::{Request, Response, StreamEvent};

// ============================================================================
// IPC Callback - 流式输出回调
// ============================================================================

/// IPC 流式回调，将事件通过 Unix Socket 发送给 Kotlin 端
struct IpcCallback<W: Write + Send> {
    writer: Arc<Mutex<W>>,
}

impl<W: Write + Send> IpcCallback<W> {
    fn new(writer: W) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
        }
    }
    
    fn send_event(&self, event: StreamEvent) {
        let response = Response::event(event);
        let mut writer = self.writer.lock().unwrap();
        if let Err(e) = write_response(&mut *writer, &response) {
            error!("Failed to send event: {}", e);
        }
    }
}

impl<W: Write + Send> StreamCallback for IpcCallback<W> {
    fn on_token(&self, token: &str) {
        self.send_event(StreamEvent::Token { data: token.to_string() });
    }
    
    fn on_tool_start(&self, name: &str, args: &str) {
        self.send_event(StreamEvent::ToolStart {
            name: name.to_string(),
            args: args.to_string(),
        });
    }
    
    fn on_tool_end(&self, name: &str, result: &str) {
        self.send_event(StreamEvent::ToolEnd {
            name: name.to_string(),
            result: result.to_string(),
        });
    }
    
    fn on_complete(&self, _response: &str) {
        // Complete 事件在 handle_chat 中单独发送
    }
    
    fn on_error(&self, message: &str) {
        self.send_event(StreamEvent::Error { message: message.to_string() });
    }
}

/// 简单回调 - 不做实际流式输出，用于非流式调用
struct SimpleCallback;

impl StreamCallback for SimpleCallback {
    fn on_token(&self, _token: &str) {}
    fn on_tool_start(&self, _name: &str, _args: &str) {}
    fn on_tool_end(&self, _name: &str, _result: &str) {}
    fn on_complete(&self, _response: &str) {}
    fn on_error(&self, _message: &str) {}
}

// ============================================================================
// IPC Server
// ============================================================================

/// IPC 服务器
pub struct IpcServer {
    socket_path: String,
    agent: Arc<Mutex<Option<AgentLoop>>>,
    running: Arc<AtomicBool>,
}

impl IpcServer {
    /// 创建新的 IPC 服务器
    pub fn new(
        socket_path: &str, 
        agent: Arc<Mutex<Option<AgentLoop>>>,
        running: Arc<AtomicBool>
    ) -> Result<Self> {
        Ok(Self {
            socket_path: socket_path.to_string(),
            agent,
            running,
        })
    }
    
    /// 运行服务器
    pub fn run(self) -> Result<()> {
        // 绑定 Socket
        let listener = UnixListener::bind(&self.socket_path)
            .context("Failed to bind socket")?;
        
        // 设置权限 (允许任何进程访问)
        std::fs::set_permissions(&self.socket_path, 
            std::fs::Permissions::from_mode(0o777))
            .context("Failed to set socket permissions")?;
        
        info!("IPC server listening on {}", self.socket_path);
        
        // 接受连接
        while self.running.load(Ordering::SeqCst) {
            // 设置非阻塞接受
            listener.set_nonblocking(true)?;
            
            match listener.accept() {
                Ok((stream, addr)) => {
                    info!("New connection from {:?}", addr);
                    let agent = self.agent.clone();
                    thread::spawn(move || {
                        if let Err(e) = handle_connection(stream, agent) {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // 没有新连接，短暂休眠
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
        
        // 清理
        let _ = std::fs::remove_file(&self.socket_path);
        info!("IPC server stopped");
        
        Ok(())
    }
}

/// 处理单个连接
fn handle_connection(stream: UnixStream, agent: Arc<Mutex<Option<AgentLoop>>>) -> Result<()> {
    stream.set_nonblocking(false)?;
    
    let reader = BufReader::new(&stream);
    let mut writer = stream.try_clone()?;
    
    for line in reader.lines() {
        let line = line.context("Failed to read line")?;
        
        if line.is_empty() {
            continue;
        }
        
        // 解析请求
        let request: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let response = Response::error(0, format!("Invalid request: {}", e));
                write_response(&mut writer, &response)?;
                continue;
            }
        };
        
        info!("Request: {} (id={})", request.method, request.id);
        
        // 处理请求
        match request.method.as_str() {
            "call_tool" => handle_call_tool(request, agent.clone(), &mut writer)?,
            "abort" => handle_abort(request, agent.clone(), &mut writer)?,
            "get_status" => handle_get_status(request, agent.clone(), &mut writer)?,
            "chat" => handle_chat(request, agent.clone(), &mut writer)?,
            "callback_result" => {
                let response = Response::result(request.id, serde_json::json!({"success": true}));
                write_response(&mut writer, &response)?;
            }
            _ => {
                let response = Response::error(request.id, format!("Unknown method: {}", request.method));
                write_response(&mut writer, &response)?;
            }
        }
    }
    
    Ok(())
}

/// 处理聊天请求
fn handle_chat(request: Request, agent: Arc<Mutex<Option<AgentLoop>>>, writer: &mut impl Write) -> Result<()> {
    let params = &request.params;
    let user_message = params["message"].as_str()
        .context("Missing 'message' parameter")?;
    
    // 解析历史消息
    let history: Vec<ChatMessage> = params.get("history")
        .and_then(|h| serde_json::from_value(h.clone()).ok())
        .unwrap_or_default();
    
    // 创建回调用于流式输出
    let writer_clone = writer.try_clone()?;
    let callback = IpcCallback::new(writer_clone);
    
    // 创建 tokio runtime 来执行异步调用
    let rt = Runtime::new().context("Failed to create tokio runtime")?;
    
    // 获取 agent 并执行对话
    let result = {
        let agent_guard = agent.lock().unwrap();
        if let Some(agent) = agent_guard.as_ref() {
            rt.block_on(async {
                agent.run(user_message, &history, &callback).await
            })
        } else {
            Err(peng_core::Error::Other("Agent not initialized".to_string()))
        }
    };
    
    // 发送最终响应
    match result {
        Ok(response) => {
            // 发送 Complete 事件
            let event = StreamEvent::Complete { data: response };
            let response = Response::event(event);
            write_response(writer, &response)?;
        }
        Err(e) => {
            let event = StreamEvent::Error { message: e.to_string() };
            let response = Response::event(event);
            write_response(writer, &response)?;
        }
    }
    
    Ok(())
}

/// 处理工具调用请求
fn handle_call_tool(request: Request, agent: Arc<Mutex<Option<AgentLoop>>>, writer: &mut impl Write) -> Result<()> {
    let params = &request.params;
    let tool_name = params["name"].as_str()
        .context("Missing 'name' parameter")?;
    let tool_params = params["params"].as_str()
        .unwrap_or("{}");
    
    // 创建 tokio runtime 来执行异步调用
    let rt = Runtime::new().context("Failed to create tokio runtime")?;
    
    // 使用简单回调
    let callback = SimpleCallback;
    
    // 获取 agent 并调用工具
    let result = {
        let agent_guard = agent.lock().unwrap();
        if let Some(agent) = agent_guard.as_ref() {
            // 使用 tokio runtime 执行异步工具调用
            rt.block_on(async {
                agent.call_tool(tool_name, tool_params).await
            })
        } else {
            Err(peng_core::Error::Other("Agent not initialized".to_string()))
        }
    };
    
    let response = match result {
        Ok(result_str) => {
            info!("Tool {} executed successfully, result length: {}", tool_name, result_str.len());
            Response::result(request.id, serde_json::json!({
                "success": true,
                "result": result_str
            }))
        }
        Err(e) => {
            warn!("Tool {} execution failed: {}", tool_name, e);
            Response::error(request.id, e.to_string())
        }
    };
    
    write_response(writer, &response)?;
    Ok(())
}

/// 处理中止请求
fn handle_abort(request: Request, agent: Arc<Mutex<Option<AgentLoop>>>, writer: &mut impl Write) -> Result<()> {
    let agent_guard = agent.lock().unwrap();
    if let Some(agent) = agent_guard.as_ref() {
        agent.abort();
    }
    drop(agent_guard);
    
    let response = Response::result(request.id, serde_json::json!({"success": true}));
    write_response(writer, &response)?;
    Ok(())
}

/// 处理获取状态请求
fn handle_get_status(request: Request, agent: Arc<Mutex<Option<AgentLoop>>>, writer: &mut impl Write) -> Result<()> {
    let status_json: serde_json::Value = {
        let agent_guard = agent.lock().unwrap();
        if let Some(agent) = agent_guard.as_ref() {
            let status: KernelStatus = agent.get_status();
            serde_json::to_value(status).unwrap_or_else(|_| {
                serde_json::json!({"initialized": true, "message": "运行中"})
            })
        } else {
            serde_json::json!({
                "initialized": false,
                "model": "",
                "message": "内核尚未初始化"
            })
        }
    };
    
    let response = Response::result(request.id, status_json);
    write_response(writer, &response)?;
    Ok(())
}

/// 写入响应
fn write_response(writer: &mut impl Write, response: &Response) -> Result<()> {
    let json = serde_json::to_string(response)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}
