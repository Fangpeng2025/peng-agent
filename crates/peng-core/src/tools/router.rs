//! 工具路由核心模块
//!
//! 定义 [`ToolHandler`] 特征和 [`ToolRouter`] 路由器。
//! 路由器维护一个工具名 → 领域的映射表，将 LLM 发起的工具调用
//! 快速路由到对应的领域处理器。

use std::collections::HashMap;

use crate::config::KernelConfig;
use crate::llm::ToolDefinition;
use crate::types::{Error, Result, ToolDomain};

use super::code::CodeHandler;
use super::file::FileHandler;
use super::media::MediaHandler;
use super::memory::MemoryHandler;
use super::phone::PhoneHandler;
use super::web::WebHandler;

// ============================================================================
// ToolHandler 特征
// ============================================================================

/// 工具处理器特征
///
/// 每个领域实现此特征，提供：
/// - 所属领域标识
/// - 工具定义列表（供 LLM 了解可调用工具）
/// - 工具执行方法
pub trait ToolHandler: Send + Sync {
    /// 返回处理器所属的工具领域
    fn domain(&self) -> ToolDomain;

    /// 执行指定工具
    ///
    /// # 参数
    /// - `tool_name`: 工具名称
    /// - `params`: 工具参数（JSON 字符串）
    ///
    /// # 返回
    /// 工具执行结果字符串
    fn execute(&self, tool_name: &str, params: &str) -> Result<String>;

    /// 返回该处理器提供的所有工具定义
    fn tool_definitions(&self) -> Vec<ToolDefinition>;
}

// ============================================================================
// ToolRouter
// ============================================================================

/// 工具路由器
///
/// 维护六大领域的处理器，以及一个工具名 → 领域的快速查找表。
/// 当 LLM 发起工具调用时，路由器根据工具名查表找到对应领域，
/// 然后委派给该领域的处理器执行。
pub struct ToolRouter {
    /// 领域 → 处理器映射
    handlers: HashMap<ToolDomain, Box<dyn ToolHandler>>,
    /// 工具名 → 领域映射（用于快速路由）
    tool_map: HashMap<String, ToolDomain>,
}

impl ToolRouter {
    /// 创建工具路由器，初始化全部六大领域处理器
    ///
    /// # 参数
    /// - `config`: 内核配置，传递给各处理器以获取运行参数
    ///
    /// # 返回
    /// 初始化成功返回 ToolRouter，失败返回 Error
    pub fn new(config: &KernelConfig) -> Result<Self> {
        let mut router = Self {
            handlers: HashMap::new(),
            tool_map: HashMap::new(),
        };

        // 注册六大领域处理器
        router.register_handler(Box::new(CodeHandler::new(config)));
        router.register_handler(Box::new(MediaHandler::new(config)));
        router.register_handler(Box::new(FileHandler::new(config)));
        router.register_handler(Box::new(PhoneHandler::new(config)));
        router.register_handler(Box::new(WebHandler::new(config)));
        router.register_handler(Box::new(MemoryHandler::new(config)));

        log::info!(
            "工具路由器初始化完成: {} 个领域, {} 个工具",
            router.handlers.len(),
            router.tool_map.len()
        );

        Ok(router)
    }

    /// 注册（或替换）一个领域处理器
    ///
    /// 如果该领域已有处理器，将被替换。同时更新工具名映射表。
    ///
    /// # 参数
    /// - `handler`: 实现了 [`ToolHandler`] 的处理器
    pub fn register_handler(&mut self, handler: Box<dyn ToolHandler>) {
        let domain = handler.domain();

        // 如果是替换已有处理器，先清除旧的工具名映射
        if let Some(old_handler) = self.handlers.get(&domain) {
            for def in old_handler.tool_definitions() {
                self.tool_map.remove(&def.function.name);
            }
        }

        // 注册新的工具名映射
        for def in handler.tool_definitions() {
            self.tool_map.insert(def.function.name.clone(), domain);
        }

        log::debug!("注册工具处理器: 领域={:?}, 工具数={}", domain, handler.tool_definitions().len());

        self.handlers.insert(domain, handler);
    }

    /// 执行工具调用
    ///
    /// 根据工具名查找所属领域，然后委派给对应处理器执行。
    ///
    /// # 参数
    /// - `tool_name`: 工具名称
    /// - `params`: 工具参数（JSON 字符串）
    ///
    /// # 返回
    /// 工具执行结果；若工具名未知则返回 Error::Tool
    pub fn execute(&self, tool_name: &str, params: &str) -> Result<String> {
        let domain = self
            .tool_map
            .get(tool_name)
            .ok_or_else(|| Error::Tool(format!("未知工具: {tool_name}")))?;

        let handler = self
            .handlers
            .get(domain)
            .ok_or_else(|| Error::Tool(format!("领域处理器缺失: {domain:?}")))?;

        log::debug!("工具路由: {tool_name} → {domain:?}");

        handler.execute(tool_name, params)
    }

    /// 收集所有处理器提供的工具定义
    ///
    /// 用于在 LLM 请求中声明可调用的工具列表。
    ///
    /// # 返回
    /// 全部工具定义的列表
    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        let mut defs = Vec::new();
        for handler in self.handlers.values() {
            defs.extend(handler.tool_definitions());
        }
        defs
    }

    /// 查询指定工具名所属的领域
    ///
    /// # 参数
    /// - `tool_name`: 工具名称
    ///
    /// # 返回
    /// 工具所属领域，若未知则返回 None
    pub fn lookup_domain(&self, tool_name: &str) -> Option<ToolDomain> {
        self.tool_map.get(tool_name).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> KernelConfig {
        KernelConfig::from_json(r#"{"data_dir": "/tmp/peng_test"}"#).unwrap()
    }

    #[test]
    fn test_router_new() {
        let config = test_config();
        let router = ToolRouter::new(&config).unwrap();
        assert_eq!(router.handlers.len(), 6);
        // 3 code + 4 media + 6 file + 11 phone + 3 web + 5 memory = 32
        assert!(!router.tool_map.is_empty());
    }

    #[test]
    fn test_router_lookup_domain() {
        let config = test_config();
        let router = ToolRouter::new(&config).unwrap();
        assert_eq!(router.lookup_domain("execute_shell"), Some(ToolDomain::Code));
        assert_eq!(router.lookup_domain("read_file"), Some(ToolDomain::File));
        assert_eq!(router.lookup_domain("send_notification"), Some(ToolDomain::Phone));
        assert_eq!(router.lookup_domain("web_fetch"), Some(ToolDomain::Web));
        assert_eq!(router.lookup_domain("save_memory"), Some(ToolDomain::Memory));
        assert_eq!(router.lookup_domain("generate_image"), Some(ToolDomain::Media));
        assert_eq!(router.lookup_domain("nonexistent_tool"), None);
    }

    #[test]
    fn test_router_execute_unknown_tool() {
        let config = test_config();
        let router = ToolRouter::new(&config).unwrap();
        let result = router.execute("nonexistent_tool", "{}");
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Tool(msg) => assert!(msg.contains("未知工具")),
            _ => panic!("期望 Tool 错误"),
        }
    }

    #[test]
    fn test_router_get_tool_definitions() {
        let config = test_config();
        let router = ToolRouter::new(&config).unwrap();
        let defs = router.get_tool_definitions();
        assert!(!defs.is_empty());
        // 验证所有定义都有名称
        for def in &defs {
            assert!(!def.function.name.is_empty());
            assert!(!def.function.description.is_empty());
        }
    }
}
