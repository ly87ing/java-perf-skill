---
name: java-perf
description: Diagnoses Java performance issues using AST analysis and LSP reasoning. Identifies N+1 queries, memory leaks, lock contention, and concurrency risks. 触发词：性能问题, 分析性能, 性能排查, 性能分析, 性能优化, 响应慢, CPU高, 内存暴涨, 内存溢出, OOM, GC频繁, 连接池满, 线程池满, 超时, 消息积压, 卡顿, 延迟高, 占用高. Keywords: performance issue, slow response, high CPU, memory spike, GC pressure, resource exhaustion, troubleshoot performance, deadlock.
allowed-tools: Bash, Read, mcp__cclsp__find_definition, mcp__cclsp__find_references
---

# Java Performance Expert (Radar-Sniper Protocol)

> **核心原则**：知识预加载 → 雷达扫描（0 Token）→ 狙击验证（LSP 推理）→ 影响评估

---

## Phase -1: 检查 CLI 可用性

> [!CRITICAL]
> **执行任何 java-perf 命令前，必须先检查 CLI 是否可用。**

```bash
# 检查 java-perf 是否已安装
which java-perf || echo "NOT_INSTALLED"
```

**如果返回 `NOT_INSTALLED`**，执行以下安装命令：

```bash
# 安装 java-perf CLI
bash "${CLAUDE_PLUGIN_ROOT}/scripts/install-binary.sh"
# 添加到 PATH（当前 session）
export PATH="$HOME/.local/bin:$PATH"
```

安装完成后重新检查：
```bash
java-perf --version
```

---

## CLI 命令

```bash
# 雷达扫描 - 全项目 AST 分析
java-perf scan --path ./src

# 显示完整结果（含 P1）
java-perf scan --path ./src --full

# 单文件分析
java-perf analyze --file ./Foo.java

# 检查清单（根据症状）
java-perf checklist --symptoms memory,cpu

# 反模式列表
java-perf antipatterns

# 项目摘要
java-perf summary --path ./

# JDK 工具
java-perf jstack --pid 12345
java-perf jmap --pid 12345
java-perf javap --class ./Target.class

# 日志分析
java-perf log --file ./app.log
```

> [!TIP]
> CLI 默认输出 **人类可读的 Markdown**，无需解析。

---

## Phase 0: 🧠 知识预加载

> [!CRITICAL]
> Session 启动时 Hook 会自动运行 `java-perf summary`。优先阅读其输出中的 **Strategy Hint**（如 "WebFlux project detected"），调整分析重点。

```bash
# 症状明确时
java-perf checklist --symptoms memory

# 通用分析
java-perf antipatterns
```

---

## Phase 1: 🛰️ 雷达扫描

> [!IMPORTANT]
> **必须先执行雷达扫描**，不要直接 grep 搜索。

```bash
java-perf scan --path ./
```

返回：P0/P1 分类的嫌疑点列表

---

## Phase 2: 🎯 狙击验证

> [!CAUTION]
> **只跳转到雷达标记的位置**，使用 LSP 推理验证。

### 验证步骤

1. **跳转到嫌疑位置**
   ```
   mcp__cclsp__find_definition({ file_path: "UserService.java", symbol_name: "findById" })
   ```

2. **读取关键代码（限 50 行）**
   ```
   Read file: UserService.java (lines 100-150)
   ```

3. **执行推理验证**

| 嫌疑类型 | 推理问题 | 验证方法 |
|----------|----------|----------|
| N+1 | 被调用方法是 DAO/RPC 吗？ | LSP 跳转检查 @Repository/@FeignClient |
| ThreadLocal | 有配对的 remove() 吗？ | 搜索同方法内 `.remove()` |
| 锁竞争 | 临界区内有 IO 吗？ | 检查 synchronized 块内代码 |
| 无界缓存 | 有 TTL/maximumSize 吗？ | 查找 `.expireAfter`/`.maximumSize` |
| 嵌套循环 | 集合规模多大？ | 推理 N×M 量级 |

---

## Phase 3: 🔬 法医取证（可选）

| 场景 | 命令 |
|------|------|
| 线程死锁/阻塞 | `java-perf jstack --pid 12345` |
| 字节码锁分析 | `java-perf javap --class ./Target.class` |
| 堆内存分析 | `java-perf jmap --pid 12345` |
| 日志异常归类 | `java-perf log --file ./app.log` |

---

## Phase 4: 📊 影响评估

> [!IMPORTANT]
> **每个问题必须量化影响**

| 维度 | 公式 | 示例 |
|------|------|------|
| 放大系数 | 循环次数 × 单次耗时 | 100次 × 10ms = 1秒 |
| 内存增长 | 对象大小 × 频率 × 存活时间 | 1KB × 1000/分钟 × 无TTL = 1.4GB/天 |
| 并发影响 | 锁粒度 × 持有时间 × 并发数 | 方法级锁 × 100ms × 200并发 = 串行等待 |

---

## 输出格式

```
📍 **位置**：`文件:行号`
🔍 **根因**：为什么有问题（附推理过程）
📊 **影响**：量化的放大倍数/内存增长/并发瓶颈
🔧 **修复**：可直接应用的代码 Patch
```

---

## 症状快速定位

| 症状 | 雷达检测 | 狙击验证 | 影响评估 |
|------|----------|----------|----------|
| 内存 | ThreadLocal, static Map | 检查 remove/TTL | 内存增长速率 |
| CPU | synchronized, 循环 | 锁范围/复杂度 | 等待时间 |
| 响应慢 | 循环内调用 | 确认 DAO/RPC | 放大系数 |
| 资源 | Executors.new | 是否有界 | 峰值线程数 |

---

## 附加资源

- 完整规则列表: [RULES.md](RULES.md)
- CLI 规则查询: `java-perf antipatterns`
