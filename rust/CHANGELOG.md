# Changelog

所有重要变更记录在此文件中。

## [9.5.0] - 2025-12-27

### 新增
- **版本同步脚本**: `scripts/sync-version.sh` 确保版本号一致性
- **CI 版本检查**: `.github/workflows/version-check.yml` 自动验证版本同步
- **Import 解析基础**: `extract_imports()` 方法 + `import_query` 预编译

### 修复
- 修复 `tree_sitter_java.rs` 中 `extract_imports` 重复定义问题
- 统一 SKILL.md 版本号至 v9.4.0

### 技术
- 48 个测试用例全部通过
- 代码清理，移除重复函数

## [9.4.0] - 2025-12-27

### 新增
- **CallGraph 污点分析**: Phase 1 同时构建 SymbolTable + CallGraph
- **N+1 验证增强**: `NPlusOneHandler` 使用 `trace_to_layer` 验证调用链
- **serde_yaml 配置解析**: 结构化 Spring 配置分析
- **Query 外部化**: `include_str!` 加载 `resources/queries/*.scm`

### 改进
- SymbolTable 并行合并 (Rayon reduce)
- 版本号动态获取 `env!("CARGO_PKG_VERSION")`
- `RuleContext` 扩展 `call_graph` 字段

### 技术
- 48 个测试用例全部通过

## [9.3.0] - 2025-12-26

### 新增
- **RuleHandler trait**: 多态分发替代巨型 match
- **预编译 Query**: 一次编译，多次使用

## [8.0.0] - 2025-12-26

### 新增
- **Two-Pass 架构**: Indexing → Analysis
- **语义分析**: SymbolTable 跨文件类型追踪
- **动态 Skill 策略**: 基于项目技术栈

## [6.0.0] - 2025-12-26

### 变更
- 移除 MCP 模式，采用纯 CLI + Skill 架构

## [5.2.0] - 2025-12-25

### 新增
- Tree-sitter AST 分析引擎
- N+1、嵌套循环、ThreadLocal 泄漏检测
