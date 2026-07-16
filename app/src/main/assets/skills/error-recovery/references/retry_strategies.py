# Retry Strategies Library

通用重试策略实现，可直接复用。

## Python 实现

### 指数退避重试

```python
import time
import random
import logging
from typing import Callable, TypeVar, Optional, Tuple, Any
from functools import wraps

logger = logging.getLogger(__name__)
T = TypeVar('T')

class RetryConfig:
    """重试配置"""
    def __init__(
        self,
        max_retries: int = 3,
        base_delay: float = 1.0,
        max_delay: float = 60.0,
        backoff_multiplier: float = 2.0,
        jitter: bool = True,
        retryable_exceptions: Tuple[type, ...] = (Exception,),
        on_retry: Optional[Callable[[int, Exception, float], None]] = None,
    ):
        self.max_retries = max_retries
        self.base_delay = base_delay
        self.max_delay = max_delay
        self.backoff_multiplier = backoff_multiplier
        self.jitter = jitter
        self.retryable_exceptions = retryable_exceptions
        self.on_retry = on_retry

def calculate_delay(
    attempt: int,
    base_delay: float,
    max_delay: float,
    backoff_multiplier: float,
    jitter: bool = True,
) -> float:
    """计算下次重试延迟"""
    delay = min(base_delay * (backoff_multiplier ** attempt), max_delay)
    if jitter:
        delay *= (0.5 + random.random() * 0.5)
    return delay

def retry(
    func: Callable[..., T],
    config: Optional[RetryConfig] = None,
) -> Callable[..., T]:
    """
    装饰器：为函数添加重试能力
    
    用法:
        @retry(config=RetryConfig(max_retries=3))
        def fetch_data():
            ...
    """
    if config is None:
        config = RetryConfig()
    
    @wraps(func)
    def wrapper(*args, **kwargs) -> T:
        last_exception = None
        for attempt in range(config.max_retries + 1):
            try:
                return func(*args, **kwargs)
            except config.retryable_exceptions as e:
                last_exception = e
                if attempt == config.max_retries:
                    break
                delay = calculate_delay(
                    attempt, config.base_delay, config.max_delay,
                    config.backoff_multiplier, config.jitter
                )
                logger.warning(
                    f"Attempt {attempt+1}/{config.max_retries+1} failed: {e}. "
                    f"Retrying in {delay:.1f}s..."
                )
                if config.on_retry:
                    config.on_retry(attempt, e, delay)
                time.sleep(delay)
        raise last_exception
    return wrapper

def retry_with_result(
    func: Callable[..., T],
    config: Optional[RetryConfig] = None,
) -> Callable[..., Tuple[bool, T, Optional[Exception]]]:
    """
    装饰器：返回 (success, result, error) 元组
    调用方可以自行决定如何处理失败
    """
    if config is None:
        config = RetryConfig()
    
    @wraps(func)
    def wrapper(*args, **kwargs) -> Tuple[bool, T, Optional[Exception]]:
        last_exception = None
        result = None
        for attempt in range(config.max_retries + 1):
            try:
                result = func(*args, **kwargs)
                return (True, result, None)
            except config.retryable_exceptions as e:
                last_exception = e
                if attempt == config.max_retries:
                    break
                delay = calculate_delay(
                    attempt, config.base_delay, config.max_delay,
                    config.backoff_multiplier, config.jitter
                )
                logger.warning(f"Attempt {attempt+1} failed: {e}. Retry in {delay:.1f}s")
                time.sleep(delay)
        return (False, result, last_exception)
    return wrapper
```

### 快速使用示例

```python
# 示例1: 基本重试
@retry(config=RetryConfig(max_retries=3, base_delay=1.0))
def call_external_api(url):
    import requests
    return requests.get(url)

# 示例2: 只重试特定异常
@retry(config=RetryConfig(
    max_retries=5,
    base_delay=2.0,
    max_delay=30.0,
    retryable_exceptions=(ConnectionError, TimeoutError),
))
def fetch_with_timeout(url):
    import requests
    return requests.get(url, timeout=10)

# 示例3: 带回调的通知
def on_retry_callback(attempt, error, delay):
    print(f"Retry #{attempt}: {error} (next in {delay:.1f}s)")

@retry(config=RetryConfig(
    max_retries=3,
    on_retry=on_retry_callback,
))
def unstable_operation():
    ...

# 示例4: 函数式调用
def do_work():
    return retry(
        lambda: external_service.get_data(),
        RetryConfig(max_retries=2, base_delay=0.5)
    )()
```

## 错误分类工具

```python
def classify_error(error: Exception) -> str:
    """
    根据异常类型和消息分类错误
    
    Returns:
        'temporary' | 'permanent' | 'rate_limit' | 'auth' | 'data' | 'resource'
    """
    msg = str(error).lower()
    
    # 网络/临时性错误
    if any(kw in msg for kw in ['timeout', 'connection', 'refused', 'reset', 'unavailable']):
        return 'temporary'
    
    # 限流
    if any(kw in msg for kw in ['rate limit', 'too many requests', '429']):
        return 'rate_limit'
    
    # 认证/权限
    if any(kw in msg for kw in ['permission denied', 'unauthorized', '401', 'forbidden', '403', 'auth']):
        return 'auth'
    
    # 资源不足
    if any(kw in msg for kw in ['out of memory', 'disk full', 'no space', 'too many open files']):
        return 'resource'
    
    # 数据错误
    if any(kw in msg for kw in ['validation', 'bad request', '400', 'invalid']):
        return 'data'
    
    # 默认：视为临时错误（保守策略）
    return 'temporary'

def should_retry(error: Exception, error_type: str = None) -> bool:
    """判断是否应该重试"""
    if error_type is None:
        error_type = classify_error(error)
    
    # 永久错误不重试
    if error_type in ('permanent', 'data', 'auth'):
        return False
    
    # 临时性错误、限流、资源不足都可以重试
    return True
```
