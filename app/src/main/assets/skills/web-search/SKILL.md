---
name: web-search
description: 网络搜索技能 - 使用SearXNG搜索引擎查询网络信息
trigger:
  - contains: 搜索
  - contains: 查一下
  - contains: 百度
  - contains: Google
tools:
  - name: web_search
    description: 使用SearXNG进行网络搜索
    parameters:
      type: object
      properties:
        query:
          type: string
          description: 搜索关键词
        limit:
          type: integer
          description: 返回结果数量，默认5
          default: 5
      required:
        - query
    enabled: true
---

# Web Search Skill

## 功能说明

使用自建的SearXNG搜索引擎进行网络搜索，获取实时信息。

## 使用场景

- 用户询问"搜索XXX"
- 需要查询最新信息
- 需要验证事实

## 工作流程

1. 用户输入包含触发词
2. Agent调用web_search工具
3. 工具向SearXNG发送请求
4. 返回搜索结果摘要

## 配置

SearXNG地址: `http://8.147.232.175:8889/search?q={query}&format=json`

## 示例

用户: "搜索最新的AI新闻"
Agent: 
- 调用 web_search(query="最新AI新闻", limit=5)
- 返回: 找到5条相关新闻...