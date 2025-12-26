---
name: java-perf
description: Diagnoses Java performance issues. 触发词：性能问题, 分析性能, 性能排查, 性能分析, 性能优化, 响应慢, CPU高, 内存暴涨, 内存溢出, OOM, GC频繁, 连接池满, 线程池满, 超时, 消息积压, 卡顿, 延迟高, 占用高. Keywords: performance issue, slow response, high CPU, memory spike, GC pressure, resource exhaustion, troubleshoot performance.
---

# Java Performance Expert (Radar-Sniper Protocol)

> **核心原则**：雷达扫描（0 Token）→ 狙击验证（LSP）→ 法医取证（可选）

---

## Phase 1: 🛰️ 雷达扫描 (0 Token)

> [!IMPORTANT]
> **必须先执行雷达扫描**，不要直接搜索文件或使用 grep

```
mcp__java-perf__scan_source_code({
  code: "文件内容",
  filePath: "xxx.java"
})
```

**输出**：嫌疑点列表（文件:行号 + 类型）

**全局扫描**（推荐）：
```
mcp__java-perf__java_perf_investigation({
  codePath: "./",
  symptoms: ["memory", "cpu"]
})
```

---

## Phase 2: 🎯 狙击验证 (LSP)

> [!CAUTION]
> **只跳转到雷达标记的位置**，不要盲目搜索

对每个嫌疑点：

1. **使用 LSP 跳转**
```
mcp__cclsp__find_symbol({ query: "嫌疑方法名" })
```

2. **验证上下文**
   - N+1 嫌疑 → 检查被调用方法是否是 DAO
   - ThreadLocal → 检查是否有 finally { remove() }
   - 锁竞争 → 检查锁范围大小

3. **只读关键行**（限制 50 行）
```
view_file({ path: "x.java", startLine: 100, endLine: 150 })
```

---

## Phase 3: 🔬 法医取证 (可选)

仅当需要字节码或运行时分析时使用：

| 场景 | 工具 |
|------|------|
| 线程死锁 | `mcp__java-perf__analyze_thread_dump({ pid: "12345" })` |
| 字节码锁 | `mcp__java-perf__analyze_bytecode({ filePath: "x.java" })` |
| 堆内存 | `mcp__java-perf__analyze_heap({ pid: "12345" })` |
| 引擎状态 | `mcp__java-perf__get_engine_status({})` |

---

## 症状快速定位

| 症状 | 雷达检测 | 狙击验证 |
|------|----------|----------|
| **内存** | ThreadLocal、static Map | 检查 remove/TTL |
| **CPU** | synchronized、循环 | 检查锁范围/复杂度 |
| **响应慢** | 循环内调用 | 确认是否 DAO/RPC |
| **资源** | Executors.new | 检查是否有界 |

---

## 输出格式

每个问题必须包含：
1. 📍 **位置**：`文件:行号`
2. 🔍 **根因**：为什么有问题
3. 📊 **影响**：放大倍数
4. 🔧 **修复**：可直接应用的 Patch

---

## 示例

**用户**：系统内存暴涨

**Claude**：
```
# Phase 1: 雷达扫描
mcp__java-perf__java_perf_investigation({ symptoms: ["memory"] })
→ 发现 TraceStore.java:45 ThreadLocal 嫌疑

# Phase 2: 狙击验证
view_file({ path: "TraceStore.java", startLine: 40, endLine: 60 })
→ 确认无 finally remove()

# 输出报告
📍 位置：TraceStore.java:45
🔍 根因：ThreadLocal 未清理
🔧 修复：try-finally 包裹
```
