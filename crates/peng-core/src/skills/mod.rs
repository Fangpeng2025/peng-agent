//! 技能匹配与注入模块
//!
//! 提供 YAML 格式的技能定义加载、关键词匹配和系统提示词注入。
//! 技能是智能体的能力扩展单元，每个技能定义了触发模式、
//! 系统提示词增量和所需工具列表。

pub mod skill_loader;

// 重导出关键类型
pub use skill_loader::{Skill, SkillRegistry};
