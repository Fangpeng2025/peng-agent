---
name: writing-plans
version: 1.0.0
description: 先规划再动手 — 写详细实现计划，避免盲目编码
triggers:
  - 写计划
  - writing plan
  - 实现规划
---

# 写实现计划

## 何时使用

当有需求/规格说明，需要在编码前规划时使用。

## 计划结构

1. **Goal**: 一句话描述
2. **Architecture**: 2-3句方案描述
3. **Global Constraints**: 全局约束(版本、依赖、命名)
4. **File Structure**: 涉及的文件及职责
5. **Tasks**: 逐个任务，每个任务独立可测试

## 任务格式

```
### Task N: [名称]

**Files:** 创建/修改的文件
**Interfaces:** 消费/产生的接口

- [ ] Step 1: 写失败测试
- [ ] Step 2: 运行确认失败
- [ ] Step 3: 写最小实现
- [ ] Step 4: 运行确认通过
- [ ] Step 5: Commit
```

## 原则

- DRY: 不重复
- YAGNI: 不做不需要的
- TDD: 先写测试
- 频繁提交
- 每步都有实际代码，不放占位符
