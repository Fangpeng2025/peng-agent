# Fallback Patterns 降级模式

当主服务不可用时，提供不同层级的降级方案。

## 降级模式库

### 模式1: Primary-Fallback (主备切换)

```python
class PrimaryFallback:
    """主服务 + 备用服务降级"""
    
    def __init__(self, primary, fallback):
        self.primary = primary
        self.fallback = fallback
        self.fallback_active = False
    
    def execute(self, *args, **kwargs):
        # 尝试主服务
        try:
            return self.primary.execute(*args, **kwargs)
        except Exception as e:
            # 自动切换到备用
            self.fallback_active = True
            return self.fallback.execute(*args, **kwargs)
    
    @property
    def is_fallback(self):
        return self.fallback_active
```

### 模式2: Cache-First (缓存优先)

```python
class CacheFirst:
    """优先从缓存读取，缓存失效时回源"""
    
    def __init__(self, cache, source, ttl=300):
        self.cache = cache  # {get(key), set(key, value)}
        self.source = source
        self.ttl = ttl
    
    def get(self, key):
        # 先查缓存
        cached = self.cache.get(key)
        if cached and not self._is_stale(cached):
            return cached['value'], 'hit'
        
        # 回源获取
        value = self.source.fetch(key)
        self.cache.set(key, {'value': value, 'ts': time.time()})
        return value, 'miss'
    
    def _is_stale(self, entry):
        return time.time() - entry.get('ts', 0) > self.ttl
```

### 模式3: Circuit Breaker (熔断器)

```python
class CircuitBreaker:
    """
    熔断器模式
    - CLOSED: 正常通行
    - OPEN: 熔断，直接拒绝
    - HALF_OPEN: 半开，允许试探性请求
    """
    
    def __init__(self, name, failure_threshold=5, recovery_timeout=30):
        self.name = name
        self.failure_threshold = failure_threshold
        self.recovery_timeout = recovery_timeout
        self.failure_count = 0
        self.last_failure_time = None
        self.state = 'closed'  # closed | open | half_open
    
    def __call__(self, func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            if self.state == 'open':
                if self._should_attempt_reset():
                    self.state = 'half_open'
                else:
                    raise CircuitOpenError(f"Circuit {self.name} is OPEN")
            
            try:
                result = func(*args, **kwargs)
                self._on_success()
                return result
            except Exception as e:
                self._on_failure()
                raise
        
        return wrapper
    
    def _should_attempt_reset(self):
        return time.time() - self.last_failure_time > self.recovery_timeout
    
    def _on_success(self):
        self.failure_count = 0
        self.state = 'closed'
    
    def _on_failure(self):
        self.failure_count += 1
        self.last_failure_time = time.time()
        if self.failure_count >= self.failure_threshold:
            self.state = 'open'

class CircuitOpenError(Exception):
    pass
```

### 模式4: Default Value (默认值)

```python
def with_default(func, default, error_types=(Exception,)):
    """
    装饰器：失败时返回默认值
    适合非关键数据的降级
    """
    @wraps(func)
    def wrapper(*args, **kwargs):
        try:
            return func(*args, **kwargs)
        except error_types:
            return default
    return wrapper

# 使用
@with_default(default=False)
def get_feature_flag(name):
    return feature_service.get(name)

# 使用
@with_default(default=[], error_types=(ConnectionError, TimeoutError))
def get_recommendations(user_id):
    return rec_engine.fetch(user_id)
```

### 模式5: Degraded Mode (降级模式)

```python
class DegradedService:
    """
    服务降级：在故障时提供简化版本的功能
    """
    
    def __init__(self, full_service, degraded_defaults=None):
        self.full = full_service
        self.defaults = degraded_defaults or {}
        self.degraded = False
    
    def get_user_profile(self, user_id):
        try:
            return self.full.get_profile(user_id)
        except Exception:
            self.degraded = True
            return self.defaults.get('user_profile', {})
    
    def get_recommendations(self, user_id):
        try:
            return self.full.get_recs(user_id)
        except Exception:
            self.degraded = True
            return self.defaults.get('recommendations', [])
    
    def is_degraded(self):
        return self.degraded
```

## 降级策略选择指南

| 场景 | 推荐模式 | 原因 |
|------|----------|------|
| 数据库主从切换 | Primary-Fallback | 快速切换到只读副本 |
| 配置获取 | Cache-First | 配置变更不频繁，缓存足够 |
| 外部API调用 | Circuit Breaker | 避免雪崩，给上游恢复时间 |
| 非关键UI数据 | Default Value | 静默降级，不影响主流程 |
| 核心功能降级 | Degraded Service | 提供简化但可用的功能 |

## 监控与告警

```python
import time

class DegradeMonitor:
    """监控降级事件"""
    
    def __init__(self):
        self.events = []
    
    def record(self, service, from_state, to_state, reason):
        event = {
            'ts': time.time(),
            'service': service,
            'from': from_state,
            'to': to_state,
            'reason': reason,
        }
        self.events.append(event)
        # 实际使用时应发送到监控系统
        # self.send_alert(event)
    
    def get_stats(self, window=300):
        """获取最近window秒内的统计"""
        cutoff = time.time() - window
        recent = [e for e in self.events if e['ts'] > cutoff]
        return {
            'total_degrades': len([e for e in recent if e['to'] == 'degraded']),
            'total_recoveries': len([e for e in recent if e['to'] == 'normal']),
            'by_service': {},
        }
```
