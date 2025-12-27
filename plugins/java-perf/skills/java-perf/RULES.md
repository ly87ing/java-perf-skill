# 规则覆盖详情

> 此文件包含 java-perf 检测规则的完整列表。核心工作流请参考 [SKILL.md](SKILL.md)。

## P0 严重 (必须修复)

| 规则 ID | 检测范围 | 引擎 | 说明 |
|---------|----------|------|------|
| N_PLUS_ONE | for/while/foreach 循环内 DAO 调用 | AST | 数据库 N+1 查询问题 |
| NESTED_LOOP | for-for / foreach-foreach / 混合嵌套 | AST | O(N²) 复杂度 |
| SYNC_METHOD | synchronized 方法级锁 | AST | 方法级锁粒度过大 |
| THREADLOCAL_LEAK | ThreadLocal.set() 无配对 remove() | AST | 内存泄漏风险 |
| SLEEP_IN_LOCK | synchronized 块内 Thread.sleep() | AST | 持锁睡眠 |
| LOCK_METHOD_CALL | ReentrantLock.lock() 无配对 unlock() | AST | 锁泄漏 |
| UNBOUNDED_POOL | Executors.newCachedThreadPool | AST | 无界线程池 |
| STATIC_COLLECTION | static Map/List 无 TTL | AST | 无界缓存 |
| FUTURE_GET_NO_TIMEOUT | Future.get() 无超时 | AST | 永久阻塞 |
| FLUX_BLOCK | Flux/Mono.block() | AST | 响应式阻塞 |
| DOUBLE_CHECKED_LOCKING | if-sync-if 模式 | AST | DCL 反模式 |
| SYSTEM_EXIT | System.exit() 调用 | AST | JVM 意外终止 |
| RUNTIME_EXEC | Runtime.exec() | AST | 命令注入风险 |
| LIKE_LEADING_WILDCARD | LIKE '%xxx' | AST | 全表扫描 |

## P1 警告 (建议修复)

| 规则 ID | 检测范围 | 引擎 | 说明 |
|---------|----------|------|------|
| STREAM_RESOURCE_LEAK | try 块内创建流资源 | AST | 资源泄漏风险 |
| OBJECT_IN_LOOP | 循环内创建对象 | AST | GC 压力 |
| ASYNC_DEFAULT_POOL | @Async 未指定线程池 | AST | 默认线程池风险 |
| AUTOWIRED_FIELD | @Autowired 字段注入 | AST | 测试困难 |
| SUBSCRIBE_NO_ERROR | subscribe() 无 error handler | AST | 异常丢失 |
| FLUX_COLLECT_LIST | collectList() | AST | OOM 风险 |
| LOG_STRING_CONCAT | 日志字符串拼接 | AST | 性能浪费 |
| SYNC_BLOCK | synchronized 代码块 | AST | Virtual Thread Pinning |
| SELECT_STAR | SELECT * | AST | 过多数据传输 |
| STRING_CONCAT_LOOP | 循环内 += 拼接 | AST | 字符串性能 |
| SIMPLE_DATE_FORMAT | SimpleDateFormat 使用 | AST | 非线程安全 |

## 配置文件检测

| 规则 ID | 检测范围 | 文件类型 |
|---------|----------|----------|
| HIKARI_NO_TIMEOUT | HikariCP 无超时 | YAML |
| REDIS_NO_TIMEOUT | Redis 连接无超时 | YAML |
| JPA_SHOW_SQL | show-sql=true | YAML |
| ACTUATOR_EXPOSED | management 端点暴露 | YAML |

## Dockerfile 检测

| 规则 ID | 检测范围 |
|---------|----------|
| DOCKER_LATEST_TAG | 使用 :latest 标签 |
| DOCKER_NO_TAG | 未指定镜像标签 |
| DOCKER_MANY_LAYERS | 过多 RUN 层 |
| DOCKER_SENSITIVE_ENV | 敏感信息在 ENV |

---

完整规则列表可通过 CLI 获取：

```bash
java-perf antipatterns
```
