# 性能问题诊断参考

## 目录
- [快速诊断表](#快速诊断表)
- [症状诊断处方](#症状诊断处方)
- [反模式警示](#反模式警示)
- [框架陷阱](#框架陷阱)
- [诊断工具](#诊断工具)
- [分析方法](#分析方法)

---

## 快速诊断表

| 症状 | 可能原因 | 推荐模式 |
|------|----------|----------|
| 内存暴涨 | 对象创建风暴、资源泄露 | 对象池、生命周期管理 |
| CPU高 | 死循环、正则回溯、锁竞争 | 算法优化、锁分段 |
| 响应慢 | IO阻塞、锁竞争、下游慢 | 异步化、熔断、缓存 |
| 资源耗尽 | 连接池/线程池满、句柄泄露 | 资源复用、背压 |
| 服务不可用 | 雪崩、级联失败 | 熔断、降级、舱壁 |
| 消息积压 | 消费慢、突发流量 | 批量消费、并行消费 |

---

## 症状诊断处方

| 症状 | 诊断 | 处方 |
|------|------|------|
| 内存突然暴涨 | 对象创建风暴 | 对象池/结果复用 |
| 内存持续上涨 | 资源泄露 | 生命周期管理/定期清理 |
| CPU使用率高 | 计算密集/死循环 | 算法优化/分片处理 |
| CPU Load高 | 线程阻塞/IO等待 | 异步化/线程池调整 |
| 响应慢+锁竞争 | 锁粒度大 | 锁分段/读写锁 |
| 响应慢+IO阻塞 | 下游慢 | 异步化/缓存/熔断器 |
| 消息积压+消费慢 | 处理能力不足 | 批量消费/并行消费 |
| 消息积压+生产快 | 突发流量 | 背压/限流 |

---

## 反模式警示

### 1. 锁内执行 IO
```java
// ❌ 锁内网络调用
synchronized (lock) { httpClient.get(url); }
// ✅ 锁外获取，锁内只写
String result = httpClient.get(url);
synchronized (lock) { cache.put(key, result); }
```

### 2. 循环内创建对象
```java
// ❌ 每次循环创建
for (int i = 0; i < 10000; i++) { new StringBuilder(); }
// ✅ 复用对象
StringBuilder sb = new StringBuilder();
for (int i = 0; i < 10000; i++) { sb.setLength(0); }
```

### 3. 无界队列
```java
// ❌ LinkedBlockingQueue 无界
Executors.newFixedThreadPool(10);
// ✅ 有界队列 + 拒绝策略
new ThreadPoolExecutor(10, 10, 0L, TimeUnit.MILLISECONDS,
    new ArrayBlockingQueue<>(1000), new CallerRunsPolicy());
```

### 4. 缓存穿透
```java
// ❌ 并发查库
if (cache.get(id) == null) { db.query(id); }
// ✅ 加锁防穿透
synchronized (("key:" + id).intern()) { /* double-check */ }
```

### 5. N+1 放大
```java
// ❌ 循环查库
for (User u : users) { dao.getProfile(u.getId()); }
// ✅ 批量查询
dao.getProfiles(users.stream().map(User::getId).toList());
```

---

## 框架陷阱

| 框架 | 陷阱 | 描述 |
|------|------|------|
| Akka | Unbounded Mailbox | 默认邮箱无界，积压导致 OOM |
| Reactor | 无背压 | 下游消费慢时积压 |
| Netty | EventLoop阻塞 | IO线程中执行DB等耗时操作 |
| Netty | ByteBuf泄露 | ReferenceCounted 未释放 |
| Hibernate | Session膨胀 | 一级缓存持有大量对象 |

---

## 诊断工具

| 问题类型 | 工具 | 用途 |
|----------|------|------|
| 内存 | jmap, MAT, async-profiler | 堆分析、泄露定位 |
| CPU | async-profiler, perf | 火焰图、热点分析 |
| 锁/响应慢 | jstack, arthas | 线程阻塞、方法耗时 |
| 资源 | lsof, netstat | 句柄/连接监控 |
| 消息 | mqadmin, kafka-consumer-groups | 积压监控 |

---

## 分析方法

### 技术栈识别
读取 `pom.xml`/`build.gradle`，匹配关键指纹：
- `akka`, `actor` → 检查 Mailbox, Dispatcher
- `reactor`, `webflux` → 检查 Backpressure
- `netty` → 检查 ByteBuf泄露, EventLoop阻塞
- `mybatis`, `hibernate` → 检查 N+1, 缓存

### 症状关联矩阵

| 组合现象 | 根因 | 验证方向 |
|----------|------|----------|
| CPU高 + 吞吐低 | 锁竞争 | 检查 synchronized |
| CPU高 + 频繁GC | GC Thrashing | 排查内存 |
| 延迟高 + CPU低 | IO阻塞 | 检查下游超时 |
| OOM + 流量突增 | 无界队列 | 检查 LinkedBlockingQueue |

### GC 诊断

| GC 现象 | 诊断 | 策略 |
|---------|------|------|
| Full GC 后内存大降 | Memory Churn | 优化对象创建 |
| Full GC 后内存居高 | Memory Leak | 搜索 static Map |
| Young GC 极频繁 | 分配过快 | 扩容 Eden |

### 日志算术
- **放大倍数**: A = 执行次数 / 触发次数 (A > 100 为 P0 问题)
- **浪费率**: W = 1 - 有效操作数/总操作数 (W > 90% 需检查)
