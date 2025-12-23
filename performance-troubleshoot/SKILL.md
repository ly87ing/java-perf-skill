---
name: performance-troubleshoot
description: Troubleshoot performance and resource issues including slow response, high CPU, memory spikes, OOM, GC pressure, resource exhaustion, service unavailable, and message backlog. Use when user reports slow response (响应慢), high CPU (CPU高), memory surge (内存暴涨), OOM errors (内存溢出), GC issues (GC频繁), connection pool exhausted (连接池满), thread pool exhausted (线程池满), service down (服务不可用), timeout (超时), high error rate (错误率高), message backlog (消息积压), or needs performance troubleshooting (性能排查/性能分析). Applicable to API services, message queues, real-time systems, databases, and microservices.
---

# 性能问题排查 Skill

专业的性能问题排查助手，帮助开发者分析和解决各类性能与资源问题。

## 触发后响应

当用户提到性能问题时，首先询问问题类型：

```
我来帮您排查性能问题。首先请确认一下：

您遇到的是哪类问题？
1. 响应慢 - 接口延迟高、吞吐低
2. CPU问题 - CPU使用率高、负载高
3. 内存问题 - 内存暴涨、OOM、GC频繁
4. 资源耗尽 - 连接池满、线程池满、文件句柄不足
5. 服务不可用 - 宕机、超时、错误率高
6. 消息积压 - 队列积压、消费延迟
7. 其他 - 自由描述您遇到的问题
```

## 信息收集流程

**重要：使用 `AskUserQuestion` 工具分步引导用户输入，每次只问 1-3 个问题。**

## 信息收集流程

**重要：已确定问题类型后，使用 `AskUserQuestion` 工具一次性收集所有必要信息。**

## 信息收集流程

**重要：使用 `AskUserQuestion` 工具分步引导用户输入，不要一次性询问所有问题。**

**核心原则：**
1. **分步进行**：每次只进行一轮提问，等待用户回答后再进行下一轮。
2. **批量提问**：每一轮可以包含 1-3 个相关问题（使用 AskUserQuestion 的 questions 数组）。
3. **允许未知**：如果用户不清楚，允许跳过或回答“未知”。

### 场景化引导模板（参考）

#### 1. 响应慢 (Slow Response)

**第1轮：现象确认**
```
1. 是所有接口都慢，还是特定接口？
2. 是突然变慢（峰值），还是持续变慢？
```

**第2轮：量化数据**
```
1. P99 延迟是多少？正常时是多少？
2. 当前 QPS 是多少？
```

**第3轮：依赖与环境**
```
1. 下游服务（DB/Redis/API）是否有延迟报警？(选项: 是/否/未知)
2. 发生问题的环境是？(选项: 生产/测试)
```

#### 2. CPU问题 (High CPU)

**第1轮：现象确认**
```
1. CPU 使用率具体是多少？(如 >90%)
2. 是单台机器异常，还是集群普遍异常？
```

**第2轮：时机与任务**
```
1. 异常发生时，是否有定时任务或批处理在运行？
2. 是否刚刚进行了代码发布或配置变更？
```

#### 3. 内存问题 (Memory/OOM)

**第1轮：现象确认**
```
1. 是内存缓慢增长（泄露），还是突然暴涨（风暴）？
2. 是否已经抛出了 OOM (Out Of Memory) 异常？(选项: 是/否)
```

**第2轮：量化数据**
```
1. 正常内存水位 vs 异常水位是多少？
2. 堆内存配置 (Xmx) 是多少？
```

#### 4. 资源耗尽 (Resource Exhaustion)

**第1轮：资源类型**
```
1. 具体是哪种资源？(选项: 连接池/线程池/句柄/其他)
2. 报错信息是什么？
```

**第2轮：配置与状态**
```
1. 该资源的最大配置 (Max) 是多少？
2. 当前使用量是多少？持续了多久？
```

#### 5. 服务不可用/稳定性 (Stability)

**第1轮：影响范围**
```
1. 是完全宕机，还是部分请求失败？
2. 持续了多长时间？目前是否已恢复？
```

**第2轮：错误详情**
```
1. 主要的错误码或报错日志是什么？
2. 之前是否有类似情况？
```

#### 6. 消息积压 (Message Backlog)

**第1轮：积压情况**
```
1. 当前积压了多少条消息？(Lag值)
2. 积压是逐渐产生的，还是瞬间产生的？
```

**第2轮：生产/消费状态**
```
1. 生产速度是否有激增？
2. 消费端是否有报错或变慢？
```

### 步骤3：分析范围 (所有类型通用)

**第3/4轮：确定分析范围**
```
您希望我扫描哪些代码目录？（可选）
```
```
您希望如何进行代码分析？
1. 指定目录/文件（推荐，支持多个路径）
2. 扫描整个项目
3. 暂不扫描代码
```

## 深度分析策略 (Active Analysis)

**在生成报告前，必须主动执行以下步骤（严禁猜测）：**

### 1. 侦察 (Reconnaissance)
- **技术栈识别**：读取构建文件 (`pom.xml`, `go.mod`, `package.json`, `requirements.txt`, `Cargo.toml`)。
- **结构确认**：
  - 如果是 **Monorepo** (只有构建文件没代码)，**必须**先找到核心子模块 (往往在 `apps/`, `services/`, `cmd/` 下)。
  - **不要**在根目录盲目搜索，先 `cd` 到具体模块。

### 2. 搜索 (Keyword Search)
根据 [CHECKLIST.md](CHECKLIST.md) 中的映射表，搜索相关反模式。
**⚠️ 安全搜索规则 (必须遵守)**：
1. **排除无关目录**：总是带上 `--exclude-dir={node_modules,target,vendor,.git,dist,build,test}`。
2. **限制输出行数**：总是带上 `| head -n 20`，防止刷屏。

| 问题类型 | 推荐搜索模式 (grep) |
|----------|-------------------|
| **CPU问题** | `ThreadPool`, `while(true)`, `synchronized`, `json.Unmarshal` |
| **内存问题** | `static Map`, `cache.put`, `InputStream` (未关闭), `bitmap` |
| **资源耗尽** | `ConnectTimeout`, `max-threads`, `max-connections`, `ulimit` |
| **超时/慢** | `timeout`, `Thread.sleep`, `slow-query`, `%.*like`, `Full Scan` |
| **链路追踪** | `trace_id`, `span_id`, `correlation_id`, `X-Request-ID` |

### 3. 日志分析 (Log Investigation)
代码只是静态的，现场在日志里。**必须**尝试：
- **定位日志**：`find . -name "*.log" -o -name "*.out" | head -n 5`
- **错误分布**：`grep -i "error" app.log | head -n 10`
- **异常统计**：`grep "Exception" app.log | sort | uniq -c | sort -nr | head -n 5`

### 4. 配置核查 (Config Validation)
- 定位关键配置文件 (`application.properties`, `nginx.conf`, `.env`).
- 检查关键性能参数（线程池大小、内存限制、超时设置）是否合理。

### 5. 证据链 (Evidence Chain)
- **拒绝推测**：如果没有看到相关代码，不要在报告中声称“可能没释放资源”。
- **引用文件**：报告中必须引用具体的文件路径和行号 (file:line) 作为证据。

## 代码分析

参考 [CHECKLIST.md](CHECKLIST.md) 的索引和 [REFERENCE.md](REFERENCE.md) 的决策树进行审查。

## 生成报告

- 文件名: `troubleshoot-report-YYYYMMDD-问题类型.md`
- 格式: 按照 [TEMPLATE.md](TEMPLATE.md) 输出

## 任务完成

生成报告后停止并告知用户：
```
[完成] 诊断报告已生成: troubleshoot-report-xxx.md
如需进一步帮助，请告诉我。
```

## 交互原则

1. 分步引导：每次只问 1-3 个问题
2. 提供选项：问题类型等用选项
3. 自由输入：数值、路径等让用户直接输入
