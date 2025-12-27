# Java Perf v9.5.0 (Rust)

<p align="center">
  <img src="https://img.shields.io/badge/Version-9.5.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/Language-Rust-orange" alt="Rust">
  <img src="https://img.shields.io/badge/Size-1.9MB-green" alt="Binary Size">
  <img src="https://img.shields.io/badge/Dependencies-Zero-purple" alt="No Dependencies">
</p>

Java 性能诊断 CLI 工具 - **零依赖，单二进制**

> v9.5.0 特性：CallGraph 污点分析、serde_yaml 配置解析、Query 外部化

## 🚀 优势

| 指标 | Node.js (v3.x) | Rust (v6.0) |
|------|---------------|-------------|
| 安装依赖 | Node.js + npm install | **零依赖** |
| 二进制大小 | ~50MB | **1.9MB** |
| 启动时间 | ~500ms | **~5ms** |
| 内存占用 | ~50MB | **~5MB** |

## 📦 安装

请使用项目根目录的一键安装脚本：

```bash
cd ..
./install.sh
```

### 手动编译

```bash
cargo build --release
cp target/release/java-perf ~/.local/bin/
```

## 🔧 CLI 命令

```bash
# 雷达扫描 - 全项目 AST 分析
java-perf scan --path ./src

# 显示完整扫描结果（包含 P1）
java-perf scan --path ./src --full

# 单文件分析
java-perf analyze --file ./UserService.java

# 检查清单 (根据症状)
java-perf checklist --symptoms memory,cpu

# 反模式列表
java-perf antipatterns

# 日志分析
java-perf log --file ./app.log

# JDK 工具
java-perf jstack --pid 12345
java-perf jmap --pid 12345
java-perf javap --class ./Target.class

# 引擎状态
java-perf status

# JSON 输出
java-perf --json scan --path ./
```

## 🔍 检测规则 (28+)

### P0 严重

| 规则 | 描述 | 引擎 |
|------|------|------|
| `N_PLUS_ONE` | 循环内 IO/数据库调用 | Tree-sitter |
| `NESTED_LOOP` | 嵌套循环 O(N*M) | Tree-sitter |
| `SYNC_METHOD` | synchronized 方法级锁 | Tree-sitter |
| `THREADLOCAL_LEAK` | ThreadLocal 未 remove | Tree-sitter |
| `SLEEP_IN_LOCK` | synchronized 块内 Thread.sleep | Tree-sitter |
| `LOCK_METHOD_CALL` | ReentrantLock 无 finally unlock | Tree-sitter |
| `UNBOUNDED_POOL` | 无界线程池 | Regex |
| `UNBOUNDED_CACHE` | 无界缓存 static Map | Regex |
| `FUTURE_GET_NO_TIMEOUT` | Future.get() 无超时 | Regex |

### P1 警告

| 规则 | 描述 | 引擎 |
|------|------|------|
| `STREAM_RESOURCE_LEAK` | try 块内创建流资源 | Tree-sitter |
| `OBJECT_IN_LOOP` | 循环内创建对象 | Regex |
| `SYNC_BLOCK` | synchronized 大代码块 | Regex |
| `NO_TIMEOUT` | HTTP 客户端无超时 | Regex |
| `COMPLETABLE_JOIN` | CompletableFuture.join() | Regex |
| `LOG_STRING_CONCAT` | Logger 字符串拼接 | Regex |

## 🏗️ 架构

```
src/
├── main.rs         # CLI 入口
├── cli.rs          # 命令行参数解析
├── ast_engine.rs   # Tree-sitter Java AST 分析
├── checklist.rs    # 检查清单和反模式知识库
├── forensic.rs     # 日志指纹归类 (流式处理)
├── jdk_engine.rs   # JDK CLI (jstack/javap/jmap)
└── scanner/        # 扫描器模块
```

## License

MIT
