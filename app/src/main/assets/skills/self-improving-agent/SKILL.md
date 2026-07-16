---
name: self-improving-agent
version: 3.0.0
description: 自我迭代技能 — 错误学习、能力扩展、质量提升
triggers:
  - 自我改进
  - self-improvement
  - 错误学习
---

# 自我迭代技能

## 核心原则

1. **错误学习**: 记录失败模式+修复策略，避免重复犯错
2. **能力扩展**: 发现新工作流→保存为技能
3. **质量提升**: 代码审查→改进建议→实施改进

## 自我迭代流程

### 1. 错误学习

当遇到错误时:
1. 系统化调试(使用systematic-debugging技能)
2. 找到根因
3. 修复问题
4. 将学到的教训保存到memory

```
memory(action="add", target="memory", content="XX模块YY场景下会ZZ，原因是AA，解决方法是BB")
```

### 2. 能力扩展

当发现可复用的工作流:
1. 完成复杂任务(5+步)
2. 提炼步骤
3. 保存为技能

```
skill_manage(action="create", name="skill-name", content="...")
```

### 3. 质量提升

任务完成后:
1. 请求代码审查
2. 处理反馈
3. 实施改进
4. 验证改进有效

## 与Hermes对齐的能力

| Hermes能力 | 鹏实现 |
|-----------|---------|
| memory(add/replace/remove) | memory工具(target=memory/user) |
| skill_manage(create/patch/delete) | skill_manage工具 |
| cronjob(create/list/remove) | cronjob工具 |
| delegate_task | delegate_task工具 |
| session_search | session_search工具 |

## 限制

- 不自动修改用户配置
- 不自动发送消息给外部服务
- 错误学习仅保存到本地memory
- 新技能创建需要验证有效性
