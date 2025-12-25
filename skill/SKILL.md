---
name: java-perf
description: Diagnoses Java performance issues including slow response, high CPU, memory spikes, OOM, GC pressure, resource exhaustion, service unavailable, and message backlog. Use when user reports 响应慢, CPU高, 内存暴涨, 内存溢出, GC频繁, 连接池满, 线程池满, 服务不可用, 超时, 错误率高, 消息积压, or needs 性能排查/性能分析.
---

# Java 性能问题排查 Skill

## 信息收集

### 快速模式
若用户已提供 **代码路径 + 症状**，直接进入分析。

### 引导模式
若信息不足，回复：

```
收到。请告诉我：

**必填**：
- 代码路径：（留空=当前目录）
- 症状：内存暴涨 / CPU高 / 响应慢 / 资源耗尽 / 消息积压（可多选）

**可选**：
- 日志/Dump路径
```

---

## 分析流程

### Step 1: 获取检查项（MCP 工具）

> [!IMPORTANT]
> **优先使用 MCP 工具**，避免直接读取文件浪费 Token。

**症状映射**：
| 用户描述 | MCP 参数 |
|----------|----------|
| 内存暴涨/OOM | `memory` |
| CPU高 | `cpu` |
| 响应慢/超时 | `slow` |
| 连接池满/线程池满 | `resource` |
| 消息积压 | `backlog` |
| GC频繁/Stop-the-World | `gc` |

**调用顺序**：
1. `mcp__java-perf__get_diagnosis` - 快速诊断
2. `mcp__java-perf__get_checklist` - 检查项
3. `mcp__java-perf__search_code_patterns` - 搜索建议

---

### Step 2: 代码分析

> [!CAUTION]
> **优先尝试 LSP**，失败后使用 Grep（加 `head_limit: 50`）

---

### Step 3: 输出报告

**每个问题必须包含**：
1. 精确位置：`文件:行号`
2. 量化数据：调用次数、放大倍数
3. 可直接应用的修复代码

**报告模板**（通过 MCP 获取）：
```
mcp__java-perf__get_template()
```
