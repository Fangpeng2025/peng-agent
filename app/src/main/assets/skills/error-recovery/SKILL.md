---
name: error-recovery
version: 1.0.0
author: Agent
description: 错误恢复与重试技能，支持常见错误的自动识别、重试策略、降级方案
enabled: true
source: agent
created_at: 2026-06-16T12:50:28.080507009+00:00
updated_at: 2026-06-17T01:41:12.363162874+00:00
tags:
  - error-recovery
  - retry
  - fallback
  - automation
  - resilience
category: automation
---

# Error Recovery 错误恢复与重试技能

当操作、命令或外部服务调用失败时，自动执行错误识别、重试、降级和恢复流程。

## 核心工作流

```
错误发生 → 识别错误类型 → 匹配恢复策略 → 执行恢复 → 验证结果
                                                    ↓
                                               恢复失败 → 降级方案 → 报告
```

## 错误识别与分类

### 错误类型速查表

| 类别 | 特征 | 示例 | 重试策略 |
|------|------|------|----------|
| **网络错误** | timeout, connection refused, DNS, 429 | 连接超时、请求频率限制 | 指数退避重试 |
| **权限错误** | permission denied, access forbidden, 403 | 文件权限不足、API鉴权失败 | 不重试，切换凭据或请求授权 |
| **资源不足** | out of memory, disk full,  Too many open files | 内存不足、磁盘满 | 释放资源后重试 |
| **并发冲突** | deadlock, lock timeout, 409 Conflict | 数据库死锁、乐观锁冲突 | 短延迟重试 |
| **数据错误** | validation error, 400 Bad Request | 参数格式错误、必填字段缺失 | 修正数据后重试或报错 |
| **服务降级** | 503 Service Unavailable, 502 Bad Gateway | 上游服务不可用 | 切换到备用服务或返回缓存 |
| **临时性失败** | 随机失败、部分失败 | 偶发的API错误 | 有限次重试 |

## 重试策略

### 策略矩阵

| 策略 | 适用场景 | 参数 | 代码模板 |
|------|----------|------|----------|
| **不重试** | 数据错误、权限错误 | 次数=0 | 直接返回错误 |
| **固定间隔** | 服务短暂不可用 | 次数=3, 间隔=1s | 见下方 |
| **指数退避** | 网络抖动、API限流 | 次数=5, 初始=1s, 倍率=2 | 见下方 |
| **完整退避** | 复杂依赖系统 | 次数=5, 初始=1s, 倍率=2, 最大=60s | 见下方 |

### 指数退避模板

```python
import time
import random

def retry_with_backoff(func, max_retries=5, base_delay=1.0, max_delay=60.0, 
                       jitter=True, retryable_exceptions=(Exception,)):
    """
    指数退避重试
    - 第1次: base_delay
    - 第2次: base_delay * 2
    - 第n次: min(base_delay * 2^(n-1), max_delay)
    - 添加随机抖动防止惊群效应
    """
    last_exception = None
    for attempt in range(max_retries + 1):
        try:
            return func()
        except retryable_exceptions as e:
            last_exception = e
            if attempt == max_retries:
                break
            delay = min(base_delay * (2 ** attempt), max_delay)
            if jitter:
                delay *= (0.5 + random.random() * 0.5)  # ±50% 抖动
            log(f"Attempt {attempt+1} failed: {e}. Retrying in {delay:.1f}s...")
            time.sleep(delay)
    raise last_exception
```

### 重试决策树

```
错误发生？
├─ 4xx 客户端错误？
│   ├─ 400 Bad Request → 数据错误 → 修正后重试或报错
│   ├─ 401 Unauthorized → 权限错误 → 刷新令牌后重试（最多1次）
│   ├─ 403 Forbidden → 权限错误 → 不重试，切换凭据或报错
│   ├─ 404 Not Found → 数据错误 → 不重试
│   ├─ 409 Conflict → 并发冲突 → 指数退避重试
│   └─ 429 Too Many Requests → 限流 → 按Retry-After头等待
├─ 5xx 服务端错误？
│   ├─ 500 Internal Error → 临时失败 → 指数退避重试（3次）
│   ├─ 502 Bad Gateway → 服务降级 → 切换到备用服务
│   ├─ 503 Service Unavailable → 服务降级 → 切换到备用服务或返回缓存
│   └─ 504 Gateway Timeout → 临时失败 → 指数退避重试
└─ 网络错误？
    ├─ Connection Timeout → 临时失败 → 指数退避重试
    ├─ Connection Refused → 临时失败 → 指数退避重试
    ├─ DNS Resolution Failed → 临时失败 → 重试（DNS缓存可能生效）
    └─ SSL/TLS Error → 权限/配置错误 → 不重试，检查证书
```

## 降级方案

### 降级层级

| 级别 | 策略 | 适用场景 | 效果 |
|------|------|----------|------|
| **L0** | 正常执行 | 无故障 | 完整功能 |
| **L1** | 重试 | 临时性失败 | 可能恢复 |
| **L2** | 备用服务 | 主服务不可用 | 部分功能可用 |
| **L3** | 缓存数据 | 数据源不可用 | 旧数据可用 |
| **L4** | 默认值 | 无法获取数据 | 功能降级运行 |
| **L5** | 优雅退出 | 完全不可用 | 最小可用状态 |

### 降级实施模板

```python
class ServiceWithFallback:
    def __init__(self, primary, fallback, cache=None):
        self.primary = primary
        self.fallback = fallback
        self.cache = cache

    def get_data(self, key, max_retries=3):
        # L0: 尝试主服务
        try:
            return self.primary.fetch(key, retries=max_retries)
        except Exception as e:
            log(f"Primary failed: {e}")
            
        # L1: 尝试备用服务
        try:
            return self.fallback.fetch(key)
        except Exception as e:
            log(f"Fallback failed: {e}")
            
        # L2: 尝试缓存
        if self.cache:
            cached = self.cache.get(key)
            if cached:
                log("Returning cached data")
                return cached
                
        # L3: 返回默认值
        log("Returning default value")
        return self.get_default(key)
```

## 使用场景

### 场景1: API调用失败

```python
# 使用场景：外部API偶尔超时或返回5xx
result = retry_with_backoff(
    lambda: external_api.post("/data", payload),
    max_retries=3,
    base_delay=2.0,
    max_delay=30.0,
    retryable_exceptions=(ConnectionError, TimeoutError, HTTPError)
)
```

### 场景2: 数据库操作

```python
# 使用场景：数据库连接不稳定或发生死锁
result = retry_with_backoff(
    lambda: db.execute(query, params),
    max_retries=2,
    base_delay=0.5,
    max_delay=5.0,
    retryable_exceptions=(DeadlockError, ConnectionError, QueryTimeout)
)
```

### 场景3: 文件操作

```python
# 使用场景：网络文件系统或并发写冲突
def write_file_safe(path, content):
    try:
        write_file(path, content)
    except PermissionError:
        log(f"Permission denied writing {path}, trying with elevated privileges")
        # 降级：尝试不同写入方式或报告错误
    except OSError as e:
        retry_with_backoff(lambda: write_file(path, content), max_retries=2, base_delay=1.0)
```

### 场景4: 命令执行失败

```python
# 使用场景：shell命令可能因资源竞争失败
def shell_exec_safe(command, retries=2, base_delay=1.0):
    try:
        return shell_exec(command)
    except ShellError as e:
        if "resource temporarily unavailable" in str(e).lower():
            return retry_with_backoff(
                lambda: shell_exec(command),
                max_retries=retries,
                base_delay=base_delay
            )
        raise
```

## 错误恢复检查清单

遇到错误时，按以下步骤执行：

1. **记录错误** — 捕获完整的错误信息、堆栈跟踪、上下文
2. **分类错误** — 判断是临时性失败还是永久性错误
3. **检查重试条件** — 是否值得重试？最大重试次数？退避策略？
4. **执行恢复** — 按匹配的策略执行
5. **验证结果** — 恢复后验证数据/状态是否正确
6. **降级处理** — 如果重试耗尽，执行降级方案
7. **报告结果** — 记录恢复成功/失败，必要时通知

## 日志格式

```
[RETRY] attempt=1/5 error="Connection timeout" delay=1.0s
[RETRY] attempt=2/5 error="Connection timeout" delay=2.0s
[RETRY] attempt=3/5 error="Connection timeout" delay=4.0s
[RECOVERED] operation succeeded on attempt 4
```

```
[DEGRADE] primary_service=DOWN fallback=activating source=datastore
[DEGRADE] serving from cache (stale=5m)
[DEGRADE] returning default value for feature_flag=enable_new_ui
```

```
[RECOVERY] error="Permission denied" strategy="fallback_credentials" result="success"
[RECOVERY] error="Deadlock detected" strategy="retry_with_backoff" result="success"
[RECOVERY] error="503 Service Unavailable" strategy="all_exhausted" result="failed" fallback="default_value"
```

## 最佳实践

1. **区分错误类型** — 不是所有错误都适合重试，数据错误重试无意义
2. **设置合理上限** — 避免无限重试，设置最大重试次数和总超时
3. **添加抖动** — 重试时加入随机延迟，避免惊群效应
4. **记录重试** — 每次重试都要记录，便于后续分析和调优
5. **监控恢复率** — 统计不同策略的成功率，持续优化参数
6. **不要吞掉错误** — 重试失败后应明确报错，不要静默忽略
7. **降级要透明** — 使用降级方案时，应标记数据状态（如"缓存"、"默认值"）
8. **测试降级路径** — 确保降级方案在实际故障时可用

## 快速参考

| 情况 | 策略 | 重试次数 | 退避 |
|------|------|----------|------|
| 4xx 错误 | 不重试/修正后重试 | 0-1 | - |
| 5xx 错误 | 指数退避 | 3 | 1s, 2s, 4s |
| 网络超时 | 指数退避 | 5 | 1s, 2s, 4s, 8s, 16s |
| 死锁/并发冲突 | 短退避重试 | 2 | 0.5s, 1s |
| API限流(429) | 按Retry-After等待 | 1 | 按响应头 |
| 资源不足 | 释放后重试 | 2 | 2s, 4s |
| 权限错误 | 切换凭据 | 1 | - |
