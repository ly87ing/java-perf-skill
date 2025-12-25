---
name: java-perf
description: Diagnoses Java performance issues. 触发词：性能问题, 分析性能, 性能排查, 性能分析, 性能优化, 响应慢, CPU高, 内存暴涨, 内存溢出, OOM, GC频繁, 连接池满, 线程池满, 超时, 消息积压, 卡顿, 延迟高, 占用高. Keywords: performance issue, slow response, high CPU, memory spike, GC pressure, resource exhaustion, troubleshoot performance.
---

# Java 性能问题排查 Skill

## 信息收集

若用户已提供 **代码路径 + 症状**，直接进入分析。否则询问：

```
收到。请告诉我：
- 症状：内存暴涨 / CPU高 / 响应慢 / 资源耗尽 / 消息积压 / GC频繁（可多选）
- 代码路径：（留空=当前目录）
```

---

## 工具检测（重要！）

> [!IMPORTANT]
> 开始分析前，先检测 MCP 工具可用性

**检测方法**：尝试调用 `mcp__java-perf__diagnose_all`

**如果 MCP 不可用**，告知用户：

```
⚠️ 检测到 java-perf MCP 未安装

当前可用模式：
- [基础模式] 使用内置知识 + cclsp 代码搜索

如需增强诊断能力，请安装 MCP：
  git clone https://github.com/ly87ing/java-perf-skill.git
  cd java-perf-skill && ./install.sh

是否使用基础模式继续？
```

---

## 分析流程

### 模式 A: 完整模式（MCP 可用）

> [!IMPORTANT]
> **Token 优化**：使用 `scan_project` 一次获取扫描计划，避免多次往返

**Step 1: 获取扫描计划（推荐）**
```
mcp__java-perf__scan_project({
  symptoms: ["memory", "slow"]
})
```
返回：搜索命令列表 + 检查重点 + 精简报告格式

**Step 2: 按计划搜索（优先 cclsp）**
```
mcp__cclsp__find_symbol({ query: "ThreadLocal" })
mcp__cclsp__find_symbol({ query: "static Map" })
```

**Step 3: 只读关键文件（限制行数）**
```
view_file({ path: "x.java", startLine: 40, endLine: 90 })  // 只读 50 行

```

---

### 模式 B: 基础模式（无 MCP）

**Step 1: 症状分析**

根据症状确定检查重点：

| 症状 | 常见原因 | 优先检查 |
|------|----------|----------|
| **内存暴涨** | 无界缓存、大对象、ThreadLocal 泄露 | static Map、ThreadLocal |
| **CPU 高** | 锁竞争、死循环、正则回溯 | synchronized、while(true) |
| **响应慢** | N+1 查询、外部调用无超时、锁阻塞 | SQL 循环、timeout 配置 |
| **资源耗尽** | 无界线程池、连接泄露 | Executors、DataSource |
| **消息积压** | 消费者阻塞、处理太慢 | @KafkaListener 内的 IO |
| **GC 频繁** | 循环创建对象、大对象进老年代 | for 循环内 new、大数组 |

**Step 2: 代码搜索（强制使用 LSP）**

> [!CAUTION]
> **必须使用 `mcp__cclsp__find_symbol` 进行代码搜索**
> 禁止直接使用 grep，除非 cclsp 明确失败

```
# 强制使用 cclsp
mcp__cclsp__find_symbol({ query: "synchronized" })
mcp__cclsp__find_symbol({ query: "ThreadLocal" })

# 找到符号后，分析调用链
mcp__cclsp__find_call_hierarchy({ file: "x.java", line: 123, direction: "incoming" })
```

**搜索关键词**：
| 症状 | cclsp 搜索（必须） |
|------|-------------------|
| memory | `ThreadLocal`, `ConcurrentHashMap`, `static Map` |
| cpu | `synchronized`, `ReentrantLock`, `Atomic` |
| slow | `HttpClient`, `RestTemplate`, `@Transactional` |
| resource | `ThreadPoolExecutor`, `DataSource`, `newCachedThreadPool` |
| backlog | `@KafkaListener`, `@RabbitListener`, `BlockingQueue` |
| gc | `ArrayList`, `StringBuilder`, `stream` |

**仅当 cclsp 失败时**，使用 grep_search（需说明原因）：
```
// cclsp 失败原因：LSP 服务未启动
grep_search({ Query: "synchronized", SearchPath: "./", MatchPerLine: true })
```

**Step 3: 验证命令**

| 症状 | 验证命令 |
|------|----------|
| 内存 | `jmap -histo:live PID | head -20` |
| CPU | `jstack PID | grep -A 20 "BLOCKED"` |
| 锁 | `jstack PID | grep "deadlock"` |
| 慢 | `arthas: trace 类名 方法名` |
| 资源 | `lsof -p PID | wc -l` |

**Step 4: 常见问题模式**

<details>
<summary>🔥 锁竞争（CPU高 + 响应慢）</summary>

**特征**：多线程 BLOCKED 状态
**搜索**：`synchronized`, `ReentrantLock`
**验证**：`jstack | grep BLOCKED`
**修复**：减小锁粒度、读写锁分离、无锁算法

</details>

<details>
<summary>🔥 N+1 查询（响应慢）</summary>

**特征**：循环内单条 SQL
**搜索**：`for.*findById`, `forEach.*dao`
**验证**：开启 SQL 日志观察重复 SQL
**修复**：IN 批量查询、JOIN 查询

</details>

<details>
<summary>🔥 无界缓存（内存暴涨）</summary>

**特征**：static Map 只增不删
**搜索**：`static.*Map.*=.*new`
**验证**：`jmap -histo:live | grep HashMap`
**修复**：Caffeine/Guava Cache 带 TTL

</details>

<details>
<summary>🔥 ThreadLocal 泄露（内存 + OOM）</summary>

**特征**：ThreadLocal.set() 后无 remove()
**搜索**：`ThreadLocal` 检查配对
**验证**：MAT 分析 ThreadLocalMap
**修复**：finally 中 remove()

</details>

<details>
<summary>🔥 无界线程池（资源耗尽）</summary>

**特征**：newCachedThreadPool
**搜索**：`Executors.newCached`
**验证**：`arthas: thread -n 10`
**修复**：ThreadPoolExecutor 有界

</details>

---

### Step 5: 输出报告

每个问题必须包含：
1. **位置**：`文件:行号`
2. **原因**：为什么会有这个问题
3. **量化**：调用次数、放大倍数
4. **修复代码**：可直接应用

---

## 示例

### 用户
> 系统响应慢，CPU 也很高

### Claude（完整模式）
```
mcp__java-perf__diagnose_all({ symptoms: ["cpu", "slow"], priority: "P0" })
mcp__cclsp__find_symbol({ query: "synchronized" })
→ 输出修复方案
```

### Claude（基础模式）
```
分析：cpu + slow → 可能锁竞争(60%)
搜索：synchronized, ReentrantLock
验证：jstack | grep BLOCKED
→ 定位问题 → 输出修复方案
```
