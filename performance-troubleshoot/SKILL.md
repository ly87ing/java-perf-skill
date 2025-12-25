---
name: performance-troubleshoot
description: Diagnoses Java performance issues including slow response, high CPU, memory spikes, OOM, GC pressure, resource exhaustion, service unavailable, and message backlog. Use when user reports 响应慢, CPU高, 内存暴涨, 内存溢出, GC频繁, 连接池满, 线程池满, 服务不可用, 超时, 错误率高, 消息积压, or needs 性能排查/性能分析.
---

# Java 性能问题排查 Skill

## 信息收集

### 快速模式
若用户已提供 **代码路径 + 症状**，直接进入分析。

### 引导模式
若信息不足，回复：

```
收到。请一次性告诉我：

**必填**：
- 代码路径：（留空=当前目录，多个用逗号分隔）
- 症状：内存暴涨 / CPU高 / 响应慢 / 资源耗尽 / 消息积压 / 不确定（可多选）

**可选**：
- 日志/Dump路径：（多个用逗号分隔）
- 子代理 CLI：claude / gemini（默认 claude）
- 补充说明：（触发条件等）

**示例**：
简单: "当前目录，内存暴涨"
完整: "代码: ./service-a | 症状: 内存暴涨 | 日志: /logs/gc.log | CLI: gemini"
```

---

## 分析架构（智能分层，节省 Token）

```
┌─────────────────────────────────────────────────┐
│              主 Agent (编排层)                   │
│  - 接收输入、调度任务、汇总报告                   │
│  - Context 目标: < 20KB                          │
└─────────────────────┬───────────────────────────┘
                      │
      ┌───────────────┼───────────────┐
      ▼               ▼               ▼
   [cclsp]       [CLI 子代理]     [CLI 子代理]
   LSP代码分析    图片/截图        日志/Dump
   不占Context    隔离Context      隔离Context
```

**前置条件**：需要安装 cclsp MCP 服务器和对应语言的 LSP 服务器。

**Java LSP 安装**：
```bash
# macOS
brew install jdtls

# 配置 cclsp
mkdir -p ~/.config/cclsp
cat > ~/.config/cclsp/config.json << 'EOF'
{
  "servers": {
    "java": {
      "command": "jdtls",
      "args": [],
      "rootMarkers": ["pom.xml", "build.gradle", ".git"],
      "filetypes": ["java"]
    }
  }
}
EOF

# 添加 cclsp MCP 服务器到 Claude Code
claude mcp add cclsp -- npx -y cclsp@latest
```

---

## 分析流程

### Step 1: 代码分析（cclsp LSP 优先，强制检查）

#### ⚠️ 必须首先执行 LSP 可用性测试

**强制步骤**：在进行任何代码搜索之前，必须先调用 cclsp 工具测试可用性。

```
┌─────────────────────────────────┐
│  调用 mcp__cclsp__find_definition │
│  目标：任意 .java 文件中的类名     │
└───────────────┬─────────────────┘
                │
        返回结果是什么？
        │
        ├─ 工具不存在 / 连接失败
        │   └─→ 使用 Grep 后备方案（必须加 head_limit: 50 限制）
        │
        └─ 返回定义位置
            └─→ 继续使用 cclsp 进行语义分析
```

**测试命令示例**：
```
mcp__cclsp__find_definition
  filePath: <任意.java文件>
  symbolName: <文件中的类名>
  symbolType: class
```

**禁止**：未经 LSP 可用性测试直接使用 Grep/Glob 搜索代码

---

#### cclsp 可用时的分析方法

使用 cclsp MCP 服务器提供的工具进行语义分析（极大节省 Context）：

| MCP 工具 | 用途 | 示例场景 |
|----------|------|----------|
| **mcp__cclsp__find_definition** | 按符号名称和类型查找定义 | 定位类/方法的源码位置 |
| **mcp__cclsp__find_references** | 查找工作区中所有符号引用 | 追踪热点方法调用链 |
| **mcp__cclsp__get_diagnostics** | 获取文件诊断信息 | 检查编译错误和警告 |
| **mcp__cclsp__rename_symbol** | 重命名符号 | 安全重构 |

**工具参数说明**：

```yaml
# find_definition - 查找定义
mcp__cclsp__find_definition:
  filePath: "/path/to/File.java"      # 文件路径
  symbolName: "methodName"             # 符号名称
  symbolType: "method"                 # 类型: class, method, field, variable

# find_references - 查找引用
mcp__cclsp__find_references:
  filePath: "/path/to/File.java"
  symbolName: "methodName"
  symbolType: "method"

# get_diagnostics - 获取诊断
mcp__cclsp__get_diagnostics:
  filePath: "/path/to/File.java"
```

**LSP 优势**：只返回引用位置和符号信息，不读取完整文件内容，极大节省 Context。

---

#### LSP 不可用时的后备方案

**只有在 cclsp 工具不存在或连接失败后**，才能使用 Grep：

```yaml
# 使用 Grep 工具（非 bash grep 命令）
Grep:
  pattern: "static.*Map|static.*List|ThreadLocal"
  path: "/path/to/src"
  output_mode: "files_with_matches"
  head_limit: 50
```

**Grep 使用规则**：
- 必须添加 `head_limit: 50` 限制输出
- 优先使用 `files_with_matches` 模式，只返回文件名
- 找到文件后，使用 Read 工具定点读取关键代码段（指定 offset 和 limit）

---

### Step 2: 大文件处理（用 CLI 子代理，隔离 Context）

根据用户选择的 CLI 执行（默认 `claude`）：

**Claude CLI**：
```bash
claude -p "提示词" --files file1.png file2.log
```

**Gemini CLI**（使用 `@` 引用文件）：
```bash
# 单个文件
gemini "分析这张图片的错误信息 @./error/screenshot.png"

# 多个文件
gemini "分析这些日志中的异常 @./logs/app.log @./logs/gc.log"

# 整个目录
gemini "分析这个目录的代码结构 @./src/"
```

**分析任务模板**：

| 文件类型 | 提示词 |
|----------|--------|
| 图片/截图 | "分析这张图片，提取：1)错误信息 2)关键数字 3)异常堆栈 4)重复模式。限300字。" |
| 大日志 | "分析日志，关注：1)ERROR/EXCEPTION/OOM 2)重复执行模式 3)耗时异常 4)资源警告。返回关键行+次数统计。限100行。" |
| Heap/Thread Dump | "分析 dump，返回：1)Top10 内存占用类 2)线程状态分布 3)阻塞线程 4)重复调用栈。限500字。" |

### Step 3: 汇总与报告
将 Step 1 和 Step 2 的**摘要结果**汇总，按 [TEMPLATE.md](TEMPLATE.md) 输出报告。

---

## 问题类型与检查项

根据症状选择检查项（来自 [CHECKLIST.md](CHECKLIST.md)）：

| 问题类型 | 必查章节 | 关键检查点 |
|----------|----------|------------|
| **OOM/内存** | `[0]`, `[5]` | 放大效应、无界缓存 |
| **CPU 高** | `[0]`, `[1]` | 放大效应、锁竞争 |
| **响应慢** | `[2]`, `[3]` | IO阻塞、下游超时 |
| **资源耗尽** | `[4]`, `[5]` | 连接池、线程池 |
| **消息积压** | `[0]`, `[11]` | 消费能力、背压 |

---

## 输出标准

**每个问题必须包含**：
1. 精确位置：文件:行号
2. 量化数据：调用次数、放大倍数
3. 具体代码：可直接应用的修复

**禁止**：模糊描述、通用建议

---

## 报告输出

使用 `write_file` 保存报告，格式见 [TEMPLATE.md](TEMPLATE.md)。
