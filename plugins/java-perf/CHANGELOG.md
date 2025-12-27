# Java Perf v6.0.0 - CLI + Skill 架构

> 发布日期：2025-12-26

## 核心变更

v6.0.0 采用纯 CLI + Skill 模式，移除了 MCP 依赖，简化了分发和使用。

### 架构对比

```
v5.x (MCP 模式)                     v6.0.0 (CLI + Skill)
├── 需要 MCP 注册                   ├── 只需二进制 + Skill
├── mcp__java-perf__scan            ├── java-perf scan
├── JSON 输出需解析                 ├── Markdown 直接可读
└── 配置复杂                        └── 零配置
```

### 优势

| 指标 | v5.x | v6.0.0 |
|------|------|--------|
| 安装 | 需要 MCP 注册 | `./install.sh` 即可 |
| 调用 | `mcp__java-perf__*` | `java-perf scan` |
| 输出 | JSON (需解析) | Markdown (直接可用) |
| Token | ~200/次 | ~100/次 |
| 依赖 | MCP Server | 无 |

---

## 详细变更

### 1. 移除 MCP 依赖

**删除的文件：**
- `rust/src/mcp.rs` - MCP Server 实现（已删除）

**修改的文件：**
- `rust/src/main.rs` - 移除 MCP 模式
- `rust/src/cli.rs` - 移除 MCP 命令
- `rust/Cargo.toml` - 移除 MCP 注释

### 2. 简化安装脚本

**install.sh:**
- 移除 `--with-mcp` 参数
- 只安装二进制 + Skill
- 零配置，开箱即用

**update.sh:**
- 移除 `--with-mcp` 参数
- 简化更新流程

### 3. CLI 命令

```bash
# 雷达扫描 - 全项目 AST 分析
java-perf scan --path ./src
java-perf scan --path ./src --full --max-p1 10

# 单文件分析
java-perf analyze --file ./UserService.java

# 检查清单 (根据症状)
java-perf checklist --symptoms memory,cpu,slow

# 反模式列表
java-perf antipatterns

# 日志分析
java-perf log --file ./app.log

# JDK 工具
java-perf jstack --pid 12345
java-perf jmap --pid 12345
java-perf javap --class ./Target.class

# 项目摘要
java-perf summary --path ./

# 引擎状态
java-perf status

# JSON 输出
java-perf --json scan --path ./
```

### 4. 默认输出格式

**Markdown 格式（默认）：**
```
## 🛰️ 雷达扫描 (v5.1 并行)
**P0**: 2 | **P1**: 5 | **文件**: 45

### 🔴 P0 (严重)
| 位置 | 规则 | 说明 |
|------|------|------|
| UserService.java:123 | N_PLUS_ONE | 循环内调用 findById |
```

**JSON 格式（`--json`）：**
```json
{
  "success": true,
  "data": { ... }
}
```

---

## Token 节省分析

| 场景 | v5.x (JSON) | v6.0.0 (Markdown) | 节省 |
|------|-------------|-------------------|------|
| scan 无问题 | ~150 tokens | ~80 tokens | 47% |
| scan 有问题 | ~300 tokens | ~150 tokens | 50% |
| checklist | ~200 tokens | ~100 tokens | 50% |

---

## 安装方式

```bash
git clone https://github.com/ly87ing/dev-skills.git
cd dev-skills/plugins/java-perf
./install.sh
```

完成！

---

## 版本历史

- **v6.0.0** (2025-12-26): 纯 CLI + Skill 模式，移除 MCP 依赖
- **v5.3.0** (2025-12-26): 新增 8 条检测规则
- **v5.2.0**: AST 检测 (Tree-sitter)
- **v4.0.0**: Rust 实现
