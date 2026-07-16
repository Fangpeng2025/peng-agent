//! 技能加载器 — YAML 格式技能定义的加载、匹配和管理
//!
//! 支持从文件系统扫描 .yaml/.yml 技能文件，解析为 Skill 结构，
//! 根据用户消息匹配技能，构建系统提示词增量。

use std::collections::HashMap;
use std::path::Path;

use crate::types::{Error, Result};

// ============================================================================
// 技能定义
// ============================================================================

/// 技能定义
///
/// 一个技能定义了触发模式、系统提示词增量和所需工具列表。
/// 当用户消息匹配技能的触发模式时，技能的系统提示词增量
/// 会被注入到系统提示词中，同时技能声明的工具会被启用。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    /// 技能名称（唯一标识）
    pub name: String,
    /// 技能描述
    pub description: String,
    /// 触发模式列表（关键词或短语，匹配用户消息时触发）
    pub trigger_patterns: Vec<String>,
    /// 系统提示词增量（匹配时注入到系统提示词）
    pub system_prompt_addition: String,
    /// 所需工具列表（技能启用的工具名称）
    pub tools: Vec<String>,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 优先级（数值越大优先级越高）
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_enabled() -> bool {
    true
}

fn default_priority() -> i32 {
    0
}

impl Skill {
    /// 检查用户消息是否匹配此技能的触发模式
    ///
    /// 使用简单的子串匹配：如果用户消息中包含任一触发模式，
    /// 则认为匹配成功（不区分大小写）。
    ///
    /// # 参数
    /// - `message`: 用户消息
    ///
    /// # 返回
    /// 匹配成功返回 true
    pub fn matches(&self, message: &str) -> bool {
        let msg_lower = message.to_lowercase();
        self.trigger_patterns
            .iter()
            .any(|pattern| msg_lower.contains(&pattern.to_lowercase()))
    }
}

// ============================================================================
// 技能注册表
// ============================================================================

/// 技能注册表
///
/// 管理所有已加载的技能，支持从文件系统加载、匹配、安装和删除。
pub struct SkillRegistry {
    /// 技能映射（名称 → 技能）
    skills: HashMap<String, Skill>,
    /// 技能文件目录
    skills_dir: String,
}

impl SkillRegistry {
    /// 创建空的技能注册表
    ///
    /// # 参数
    /// - `skills_dir`: 技能文件目录路径
    ///
    /// # 返回
    /// 空的技能注册表
    pub fn new(skills_dir: &str) -> Result<Self> {
        // 确保技能目录存在
        if !Path::new(skills_dir).exists() {
            std::fs::create_dir_all(skills_dir)
                .map_err(|e| Error::Io(format!("创建技能目录失败: {e}")))?;
        }

        Ok(Self {
            skills: HashMap::new(),
            skills_dir: skills_dir.to_string(),
        })
    }

    /// 从技能目录加载所有技能文件
    ///
    /// 扫描 `skills_dir` 下的所有 .yaml/.yml 文件，解析为 Skill 结构。
    /// 如果某个文件解析失败，记录警告但继续加载其他文件。
    ///
    /// # 返回
    /// 加载成功返回 Ok(())，目录不存在等严重错误返回 Error
    pub fn load_skills(&mut self) -> Result<()> {
        let skills_dir = Path::new(&self.skills_dir);

        if !skills_dir.exists() {
            log::warn!("技能目录不存在: {}", self.skills_dir);
            return Ok(());
        }

        let entries = std::fs::read_dir(skills_dir)
            .map_err(|e| Error::Io(format!("读取技能目录失败: {e}")))?;

        let mut loaded_count = 0;
        let mut error_count = 0;

        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            if ext != "yaml" && ext != "yml" {
                continue;
            }

            match Self::load_skill_file(&path) {
                Ok(skill) => {
                    log::info!("技能加载成功: {}", skill.name);
                    self.skills.insert(skill.name.clone(), skill);
                    loaded_count += 1;
                }
                Err(e) => {
                    log::warn!("技能文件加载失败 {:?}: {e}", path);
                    error_count += 1;
                }
            }
        }

        log::info!(
            "技能加载完成: 成功={}, 失败={}, 总计={}",
            loaded_count,
            error_count,
            self.skills.len()
        );

        Ok(())
    }

    /// 从单个 YAML 文件加载技能
    fn load_skill_file(path: &Path) -> Result<Skill> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Io(format!("读取技能文件失败: {e}")))?;

        let skill: Skill = serde_yaml::from_str(&content)
            .map_err(|e| Error::Other(format!("解析技能 YAML 失败: {e}")))?;

        // 验证技能定义
        if skill.name.is_empty() {
            return Err(Error::Other("技能名称不能为空".to_string()));
        }

        if skill.trigger_patterns.is_empty() {
            log::warn!("技能 '{}' 没有定义触发模式", skill.name);
        }

        Ok(skill)
    }

    /// 根据用户消息匹配技能
    ///
    /// 返回所有匹配的技能，按优先级降序排列。
    /// 仅返回已启用的技能。
    ///
    /// # 参数
    /// - `message`: 用户消息
    ///
    /// # 返回
    /// 匹配的技能列表（按优先级降序）
    pub fn match_skills(&self, message: &str) -> Vec<&Skill> {
        let mut matched: Vec<&Skill> = self
            .skills
            .values()
            .filter(|s| s.enabled && s.matches(message))
            .collect();

        matched.sort_by(|a, b| b.priority.cmp(&a.priority));
        matched
    }

    /// 获取指定名称的技能
    ///
    /// # 参数
    /// - `name`: 技能名称
    pub fn get_skill(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// 启用或禁用技能
    ///
    /// # 参数
    /// - `name`: 技能名称
    /// - `enabled`: 是否启用
    pub fn toggle_skill(&mut self, name: &str, enabled: bool) -> Result<()> {
        let skill = self
            .skills
            .get_mut(name)
            .ok_or_else(|| Error::Other(format!("技能不存在: {name}")))?;

        skill.enabled = enabled;
        self.save_skill_to_file(name)?;

        log::info!("技能 '{}' 已{}", name, if enabled { "启用" } else { "禁用" });
        Ok(())
    }

    /// 从 YAML 内容安装技能
    ///
    /// 解析 YAML 内容为 Skill，添加到注册表，并保存到技能目录。
    ///
    /// # 参数
    /// - `content`: YAML 格式的技能内容
    ///
    /// # 返回
    /// 安装成功返回技能名称
    pub fn install_from_content(&mut self, content: &str) -> Result<String> {
        let skill: Skill = serde_yaml::from_str(content)
            .map_err(|e| Error::Other(format!("解析技能 YAML 失败: {e}")))?;

        if skill.name.is_empty() {
            return Err(Error::Other("技能名称不能为空".to_string()));
        }

        let name = skill.name.clone();
        self.skills.insert(name.clone(), skill);
        self.save_skill_to_file(&name)?;

        log::info!("技能安装成功: {name}");
        Ok(name)
    }

    /// 从 URL 安装技能
    ///
    /// 从指定 URL 下载 YAML 内容，然后安装为技能。
    /// 使用异步 reqwest 下载（需在 async 上下文中调用）。
    ///
    /// # 参数
    /// - `url`: 技能 YAML 文件的 URL
    ///
    /// # 返回
    /// 安装成功返回技能名称
    pub async fn install_from_url_async(&mut self, url: &str) -> Result<String> {
        let response = reqwest::get(url)
            .await
            .map_err(|e| Error::Other(format!("下载技能文件失败: {e}")))?;

        if !response.status().is_success() {
            return Err(Error::Other(format!(
                "下载技能文件失败: HTTP {}",
                response.status()
            )));
        }

        let content = response
            .text()
            .await
            .map_err(|e| Error::Other(format!("读取技能文件内容失败: {e}")))?;

        self.install_from_content(&content)
    }

    /// 删除技能
    ///
    /// 从注册表和文件系统中删除指定技能。
    ///
    /// # 参数
    /// - `name`: 技能名称
    pub fn delete_skill(&mut self, name: &str) -> Result<()> {
        self.skills
            .remove(name)
            .ok_or_else(|| Error::Other(format!("技能不存在: {name}")))?;

        // 删除文件
        let yaml_path = Path::new(&self.skills_dir).join(format!("{name}.yaml"));
        let yml_path = Path::new(&self.skills_dir).join(format!("{name}.yml"));

        if yaml_path.exists() {
            std::fs::remove_file(&yaml_path)
                .map_err(|e| Error::Io(format!("删除技能文件失败: {e}")))?;
        }
        if yml_path.exists() {
            std::fs::remove_file(&yml_path)
                .map_err(|e| Error::Io(format!("删除技能文件失败: {e}")))?;
        }

        log::info!("技能已删除: {name}");
        Ok(())
    }

    /// 列出所有技能
    pub fn list_skills(&self) -> Vec<&Skill> {
        let mut skills: Vec<&Skill> = self.skills.values().collect();
        skills.sort_by(|a, b| b.priority.cmp(&a.priority));
        skills
    }

    /// 获取所有已启用技能声明的工具列表
    pub fn get_enabled_tools(&self) -> Vec<String> {
        let mut tools = std::collections::HashSet::new();
        for skill in self.skills.values().filter(|s| s.enabled) {
            for tool in &skill.tools {
                tools.insert(tool.clone());
            }
        }
        let mut result: Vec<String> = tools.into_iter().collect();
        result.sort();
        result
    }

    /// 从匹配的技能构建系统提示词增量
    ///
    /// 将匹配技能的 system_prompt_addition 拼接为一段文本，
    /// 用于注入到系统提示词中。
    ///
    /// # 参数
    /// - `matched`: 匹配的技能列表
    ///
    /// # 返回
    /// 拼接后的系统提示词增量
    pub fn build_skill_context(&self, matched: &[&Skill]) -> String {
        if matched.is_empty() {
            return String::new();
        }

        let mut context = String::from("=== 激活的技能 ===\n");

        for skill in matched {
            context.push_str(&format!(
                "【{name}】{desc}\n{prompt}\n\n",
                name = skill.name,
                desc = skill.description,
                prompt = skill.system_prompt_addition
            ));
        }

        context.push_str("=== 技能注入结束 ===\n");
        context
    }

    /// 将技能保存到 YAML 文件
    fn save_skill_to_file(&self, name: &str) -> Result<()> {
        let skill = self
            .skills
            .get(name)
            .ok_or_else(|| Error::Other(format!("技能不存在: {name}")))?;

        let yaml_content = serde_yaml::to_string(skill)
            .map_err(|e| Error::Other(format!("序列化技能 YAML 失败: {e}")))?;

        let file_path = Path::new(&self.skills_dir).join(format!("{name}.yaml"));
        std::fs::write(&file_path, yaml_content)
            .map_err(|e| Error::Io(format!("写入技能文件失败: {e}")))?;

        Ok(())
    }

    /// 获取技能数量
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// 判断是否为空
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_skill() -> Skill {
        Skill {
            name: "video-production".to_string(),
            description: "视频制作技能".to_string(),
            trigger_patterns: vec![
                "制作视频".to_string(),
                "视频".to_string(),
                "video".to_string(),
            ],
            system_prompt_addition: "用户需要制作视频，请使用视频制作相关工具...".to_string(),
            tools: vec![
                "execute_shell".to_string(),
                "execute_python".to_string(),
                "process_video".to_string(),
            ],
            enabled: true,
            priority: 10,
        }
    }

    #[test]
    fn test_skill_matches() {
        let skill = sample_skill();
        assert!(skill.matches("帮我制作视频"));
        assert!(skill.matches("我想做一个video"));
        assert!(skill.matches("视频剪辑"));
        assert!(!skill.matches("帮我写个程序"));
    }

    #[test]
    fn test_skill_matches_case_insensitive() {
        let skill = sample_skill();
        assert!(skill.matches("VIDEO"));
        assert!(skill.matches("Video制作"));
    }

    #[test]
    fn test_skill_registry_new() {
        let dir = std::env::temp_dir().join("peng_test_skills_new");
        let _ = std::fs::remove_dir_all(&dir);
        let registry = SkillRegistry::new(dir.to_str().unwrap());
        assert!(registry.is_ok());
        assert!(registry.unwrap().is_empty());
    }

    #[test]
    fn test_skill_registry_install_from_content() {
        let dir = std::env::temp_dir().join("peng_test_skills_install");
        let _ = std::fs::remove_dir_all(&dir);
        let mut registry = SkillRegistry::new(dir.to_str().unwrap()).unwrap();

        let yaml = r#"
name: test-skill
description: 测试技能
trigger_patterns:
  - 测试
  - test
system_prompt_addition: "这是一个测试技能"
tools:
  - execute_shell
enabled: true
priority: 5
"#;
        let name = registry.install_from_content(yaml).unwrap();
        assert_eq!(name, "test-skill");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_skill_registry_match_skills() {
        let dir = std::env::temp_dir().join("peng_test_skills_match");
        let _ = std::fs::remove_dir_all(&dir);
        let mut registry = SkillRegistry::new(dir.to_str().unwrap()).unwrap();

        let yaml1 = r#"
name: skill-a
description: 技能A
trigger_patterns: ["编程", "代码"]
system_prompt_addition: "编程技能"
tools: ["execute_shell"]
priority: 5
"#;
        let yaml2 = r#"
name: skill-b
description: 技能B
trigger_patterns: ["视频", "video"]
system_prompt_addition: "视频技能"
tools: ["process_video"]
priority: 10
"#;

        registry.install_from_content(yaml1).unwrap();
        registry.install_from_content(yaml2).unwrap();

        let matched = registry.match_skills("帮我制作视频");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "skill-b");

        let matched2 = registry.match_skills("写代码");
        assert_eq!(matched2.len(), 1);
        assert_eq!(matched2[0].name, "skill-a");
    }

    #[test]
    fn test_skill_registry_toggle() {
        let dir = std::env::temp_dir().join("peng_test_skills_toggle");
        let _ = std::fs::remove_dir_all(&dir);
        let mut registry = SkillRegistry::new(dir.to_str().unwrap()).unwrap();

        let yaml = r#"
name: test-toggle
description: 测试
trigger_patterns: ["测试"]
system_prompt_addition: "测试"
tools: []
priority: 0
"#;
        registry.install_from_content(yaml).unwrap();

        assert!(registry.get_skill("test-toggle").unwrap().enabled);

        registry.toggle_skill("test-toggle", false).unwrap();
        assert!(!registry.get_skill("test-toggle").unwrap().enabled);

        // 禁用后不应匹配
        let matched = registry.match_skills("测试");
        assert!(matched.is_empty());
    }

    #[test]
    fn test_skill_registry_delete() {
        let dir = std::env::temp_dir().join("peng_test_skills_delete");
        let _ = std::fs::remove_dir_all(&dir);
        let mut registry = SkillRegistry::new(dir.to_str().unwrap()).unwrap();

        let yaml = r#"
name: test-delete
description: 测试
trigger_patterns: ["测试"]
system_prompt_addition: "测试"
tools: []
"#;
        registry.install_from_content(yaml).unwrap();
        assert_eq!(registry.len(), 1);

        registry.delete_skill("test-delete").unwrap();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_skill_registry_get_enabled_tools() {
        let dir = std::env::temp_dir().join("peng_test_skills_tools");
        let _ = std::fs::remove_dir_all(&dir);
        let mut registry = SkillRegistry::new(dir.to_str().unwrap()).unwrap();

        let yaml1 = r#"
name: skill-1
description: 技能1
trigger_patterns: ["a"]
system_prompt_addition: "1"
tools: ["execute_shell", "execute_python"]
priority: 0
"#;
        let yaml2 = r#"
name: skill-2
description: 技能2
trigger_patterns: ["b"]
system_prompt_addition: "2"
tools: ["execute_python", "process_video"]
priority: 0
"#;
        registry.install_from_content(yaml1).unwrap();
        registry.install_from_content(yaml2).unwrap();

        let tools = registry.get_enabled_tools();
        assert_eq!(tools, vec!["execute_python", "execute_shell", "process_video"]);
    }

    #[test]
    fn test_build_skill_context() {
        let dir = std::env::temp_dir().join("peng_test_skills_context");
        let _ = std::fs::remove_dir_all(&dir);
        let mut registry = SkillRegistry::new(dir.to_str().unwrap()).unwrap();

        let yaml = r#"
name: video
description: 视频技能
trigger_patterns: ["视频"]
system_prompt_addition: "请使用视频工具"
tools: ["process_video"]
priority: 10
"#;
        registry.install_from_content(yaml).unwrap();

        let matched = registry.match_skills("制作视频");
        let ctx = registry.build_skill_context(&matched);
        assert!(ctx.contains("激活的技能"));
        assert!(ctx.contains("视频技能"));
        assert!(ctx.contains("请使用视频工具"));
    }

    #[test]
    fn test_skill_registry_load_from_dir() {
        let dir = std::env::temp_dir().join("peng_test_skills_load");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // 写入两个技能文件
        let yaml1 = r#"
name: loaded-skill-1
description: 加载测试1
trigger_patterns: ["加载1"]
system_prompt_addition: "1"
tools: []
"#;
        let yaml2 = r#"
name: loaded-skill-2
description: 加载测试2
trigger_patterns: ["加载2"]
system_prompt_addition: "2"
tools: []
"#;
        std::fs::write(dir.join("skill1.yaml"), yaml1).unwrap();
        std::fs::write(dir.join("skill2.yml"), yaml2).unwrap();

        let mut registry = SkillRegistry::new(dir.to_str().unwrap()).unwrap();
        registry.load_skills().unwrap();

        assert_eq!(registry.len(), 2);
        assert!(registry.get_skill("loaded-skill-1").is_some());
        assert!(registry.get_skill("loaded-skill-2").is_some());
    }
}
