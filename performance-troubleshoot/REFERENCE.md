# 性能问题诊断参考

详细的诊断决策树、反模式警示、优化模式等参考内容。

---

## 症状→诊断→处方决策树

根据问题现象，推荐优化模式：

```mermaid
graph TD
    A["问题现象"] --> B{"类型?"}
    
    B -->|内存问题| C{"具体表现?"}
    C -->|突然暴涨| C1["对象创建风暴"]
    C -->|持续上涨| C2["资源泄露"]
    C1 --> C1R["推荐: 对象池/结果复用"]
    C2 --> C2R["推荐: 生命周期管理/定期清理"]
    
    B -->|CPU问题| D{"特征?"}
    D -->|使用率高| D1["计算密集/死循环"]
    D -->|Load高| D2["线程阻塞/IO等待"]
    D1 --> D1R["推荐: 算法优化/分片处理"]
    D2 --> D2R["推荐: 异步化/线程池调整"]

    B -->|响应慢| E{"原因?"}
    E -->|锁竞争| E1["锁粒度大"]
    E -->|IO阻塞| E2["下游慢"]
    E1 --> E1R["推荐: 锁分段/读写锁"]
    E2 --> E2R["推荐: 异步化/缓存/熔断器"]
    
    B -->|消息积压| F{"原因?"}
    F -->|消费慢| F1["处理能力不足"]
    F -->|生产快| F2["突发流量"]
    F1 --> F1R["推荐: 批量消费/并行消费"]
    F2 --> F2R["推荐: 背压/限流"]

    B -->|资源耗尽| G{"类型?"}
    G -->|连接池满| G1["连接泄露/配置小"]
    G -->|线程池满| G2["任务堆积/死锁"]
    G1 --> G1R["推荐: 泄露检测/弹性伸缩"]
    G2 --> G2R["推荐: 任务拒绝/隔离"]

    B -->|服务不可用| H{"类型?"}
    H -->|雪崩| H1["级联失败"]
    H1 --> H1R["推荐: 熔断/降级/舱壁"]
```

---

## 快速诊断表

| 症状 | 可能原因 | 推荐模式 |
|------|----------|----------|
| 内存问题 | 对象创建风暴、资源泄露 | 对象池、生命周期管理 |
| CPU问题 | 死循环、正则回溯、锁竞争 | 算法优化、锁分段 |
| 响应慢 | IO阻塞、锁竞争、下游慢 | 异步化、熔断、缓存 |
| 资源耗尽 | 连接池/线程池满、句柄泄露 | 资源复用、背压、弹性配置 |
| 服务不可用 | 雪崩、级联失败 | 熔断、降级、舱壁模式 |
| 消息积压 | 消费慢、突发流量 | 批量消费、并行消费 |

---

## 反模式警示

### 反模式 1: 锁内执行 IO 操作

```java
// [错误] 锁内执行网络调用
synchronized (lock) {
    String result = httpClient.get(url);  // 阻塞其他线程!
    cache.put(key, result);
}

// [正确] 先获取数据，再加锁
String result = httpClient.get(url);  // 锁外执行
synchronized (lock) {
    cache.put(key, result);  // 只保护写操作
}
```

### 反模式 2: 循环内创建对象

```java
// [错误] 每次循环都创建对象
for (int i = 0; i < 10000; i++) {
    StringBuilder sb = new StringBuilder();  // 创建1万个对象!
    sb.append(data[i]);
    process(sb.toString());
}

// [正确] 复用对象
StringBuilder sb = new StringBuilder();
for (int i = 0; i < 10000; i++) {
    sb.setLength(0);  // 重置而非新建
    sb.append(data[i]);
    process(sb.toString());
}
```

### 反模式 3: 无界队列

```java
// [错误] 无界队列，可能 OOM
ExecutorService executor = Executors.newFixedThreadPool(10);
// 内部使用 LinkedBlockingQueue 无界队列!

// [正确] 有界队列 + 拒绝策略
ExecutorService executor = new ThreadPoolExecutor(
    10, 10, 0L, TimeUnit.MILLISECONDS,
    new ArrayBlockingQueue<>(1000),  // 有界队列
    new ThreadPoolExecutor.CallerRunsPolicy()  // 背压
);
```

### 反模式 4: 双重检查锁错误实现

```java
// [错误] 没有 volatile
private static Instance instance;
if (instance == null) {
    synchronized (lock) {
        if (instance == null) {
            instance = new Instance();  // 可能看到未初始化完成的对象!
        }
    }
}

// [正确] 使用 volatile
private static volatile Instance instance;  // 添加 volatile
```

### 反模式 5: subList 返回视图

```java
// [错误] subList 返回的是视图，不是副本
List<T> result = list.subList(0, 10);
return result;  // 原列表变化会影响 result!

// [正确] 创建副本
List<T> result = new ArrayList<>(list.subList(0, 10));
return result;
```

---

## Mermaid 语法规范

**避免语法错误的规则：**

1. **不要使用 emoji** - 禁止使用特殊符号，用文字替代
2. **特殊字符需要引号** - 包含括号、冒号的标签用双引号包裹
3. **中文标签加引号** - 更安全

**正确示例：**
```mermaid
graph TD
    A["用户请求"] --> B{"需要处理?"}
    B -->|是| C["执行处理"]
    B -->|否| D["跳过"]
```

---

## 诊断工具推荐

| 问题类型 | 推荐工具 | 用途 |
|----------|----------|------|
| 内存问题 | jmap, MAT, async-profiler (alloc) | 堆分析、内存泄露定位 |
| CPU问题 | async-profiler (itimer/wall), perf | 火焰图、热点代码分析 |
| 响应慢/锁 | jstack, arthas (trace/stack) | 线程阻塞分析、方法耗时 |
| 资源耗尽 | lsof, netstat, arthas (dashboard) | 句柄/连接数监控 |
| 消息积压 | mqadmin, kafka-consumer-groups | 积压量 (Lag) 监控 |
| 服务不可用 | tshark, tcpdump | 网络抓包、协议分析 |
