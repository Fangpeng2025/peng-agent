---
name: knowledge-base
description: 知识库技能 - 存储和检索长期知识
trigger:
  - contains: 记住
  - contains: 存入知识库
  - contains: 查询知识库
  - contains: 知识库搜索
tools:
  - name: knowledge_create
    description: 创建知识条目
    parameters:
      type: object
      properties:
        title:
          type: string
          description: 知识标题
        content:
          type: string
          description: 知识内容
        tags:
          type: string
          description: 标签（逗号分隔）
          default: ""
      required:
        - title
        - content
    enabled: true
  
  - name: knowledge_search
    description: 搜索知识库
    parameters:
      type: object
      properties:
        query:
          type: string
          description: 搜索关键词
        limit:
          type: integer
          description: 返回结果数量
          default: 10
      required:
        - query
    enabled: true
  
  - name: knowledge_read
    description: 读取指定知识条目
    parameters:
      type: object
      properties:
        id:
          type: string
          description: 知识ID
      required:
        - id
    enabled: true
---

# Knowledge Base Skill

## 功能说明

长期知识存储和检索系统，支持创建、搜索、读取知识条目。

## 使用场景

- 用户说"记住这个知识"
- 用户说"存入知识库"
- 用户查询之前存储的知识

## 工作流程

1. 用户提出存储或查询需求
2. Agent调用对应工具
3. Rust端KnowledgeBaseManager处理
4. 返回结果

## 存储位置

`/sdcard/peng-agent/knowledge/`

## 示例

### 示例1：创建知识
用户: "记住：公司的WiFi密码是ABC123"
Agent:
- 调用 knowledge_create(
    title="公司WiFi密码",
    content="WiFi密码是ABC123",
    tags="公司,网络"
  )
- 返回: 已存入知识库，ID=xxx

### 示例2：搜索知识
用户: "查询WiFi密码"
Agent:
- 调用 knowledge_search(query="WiFi密码")
- 返回: 找到1条知识：公司WiFi密码是ABC123

### 示例3：读取知识
用户: "读取知识xxx"
Agent:
- 调用 knowledge_read(id="xxx")
- 返回: 知识详情...