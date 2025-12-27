# 改进计划 (Roadmap)

## 当前状态 (v9.5.0)

### ✅ 已完成
1. **版本同步机制**
   - `scripts/sync-version.sh` 脚本
   - `.github/workflows/version-check.yml` CI 检查
   - 版本号统一: Cargo.toml, README, CHANGELOG, SKILL.md

2. **Import 解析基础**
   - `compile_import_query()` 预编译查询
   - `extract_imports()` 方法提取 import 列表

---

## 🚧 待实现改进

### 1. 完善 Call Graph 解析 (增强跨包调用准确性)

**当前问题**:
- `extract_call_sites()` 中 `receiver` 是变量名（如 `userRepository`）
- 无法直接映射到类全限定名（如 `com.example.repository.UserRepository`）

**改进方案**:

```rust
// 新增结构
pub struct ImportIndex {
    /// 简单类名 -> 全限定名 (e.g., "UserRepository" -> "com.example.repository.UserRepository")
    simple_to_fqn: HashMap<String, String>,
    /// 包通配符导入 (e.g., "com.example.repository.*")
    wildcard_imports: Vec<String>,
}

// 在 Phase 1 中构建
impl JavaTreeSitterAnalyzer {
    pub fn extract_imports_index(&self, code: &str) -> Result<ImportIndex> {
        let imports = self.extract_imports(code)?;
        let mut index = ImportIndex::new();

        for import in imports {
            if import.ends_with(".*") {
                index.wildcard_imports.push(import.trim_end_matches(".*").to_string());
            } else {
                // "com.example.UserService" -> ("UserService", "com.example.UserService")
                let simple_name = import.rsplit('.').next().unwrap_or(&import);
                index.simple_to_fqn.insert(simple_name.to_string(), import);
            }
        }

        Ok(index)
    }
}

// 在 CallGraph 构建时使用
fn resolve_receiver(receiver: &str, import_index: &ImportIndex, fields: &[VarBinding]) -> String {
    // 1. 检查是否是字段，获取字段类型
    if let Some(field) = fields.iter().find(|f| f.name == receiver) {
        let type_name = &field.type_name;
        // 2. 查找 import 映射
        if let Some(fqn) = import_index.simple_to_fqn.get(type_name) {
            return fqn.clone();
        }
        return type_name.clone();
    }
    receiver.to_string()
}
```

**工作量**: ~2-3 小时

---

### 2. 增强 Spring Context 理解

**当前问题**:
- @Autowired 字段追踪依赖变量名和类型名
- 无法处理 @Qualifier、@Resource(name="xxx") 等复杂情况

**改进方案**:

```rust
// 扩展 VarBinding
pub struct VarBinding {
    pub name: String,
    pub type_name: String,
    pub is_field: bool,
    // 新增
    pub qualifier: Option<String>,  // @Qualifier("xxx") 或 @Resource(name="xxx")
}

// 扩展 structure_query 以捕获注解参数
let structure_query = r#"
    (field_declaration
        (modifiers
            (annotation
                name: (identifier) @ann_name
                arguments: (annotation_argument_list
                    (element_value_pair
                        key: (identifier) @key
                        value: (string_literal) @value
                    )
                )?
            )
        )?
        type: (_) @field_type
        declarator: (variable_declarator name: (identifier) @field_name)
    )
"#;
```

**工作量**: ~1-2 小时

---

### 3. 结构化配置解析

**当前问题**:
- `LineBasedConfigAnalyzer` 使用行匹配
- v9.4 引入 `serde_yaml` 但未全面迁移

**改进方案**:

```rust
// 定义配置结构
#[derive(Debug, Deserialize)]
struct SpringConfig {
    spring: Option<SpringSection>,
    server: Option<ServerSection>,
    management: Option<ManagementSection>,
}

#[derive(Debug, Deserialize)]
struct SpringSection {
    datasource: Option<DataSourceConfig>,
    redis: Option<RedisConfig>,
    jpa: Option<JpaConfig>,
}

#[derive(Debug, Deserialize)]
struct DataSourceConfig {
    url: Option<String>,
    #[serde(rename = "hikari")]
    hikari: Option<HikariConfig>,
}

#[derive(Debug, Deserialize)]
struct HikariConfig {
    #[serde(rename = "maximum-pool-size")]
    maximum_pool_size: Option<u32>,
    #[serde(rename = "connection-timeout")]
    connection_timeout: Option<u64>,
}

// 结构化检测
fn analyze_yaml_structured(content: &str, file: &str) -> Vec<Issue> {
    let config: SpringConfig = serde_yaml::from_str(content)?;
    let mut issues = Vec::new();

    // 检测连接池配置
    if let Some(spring) = &config.spring {
        if let Some(ds) = &spring.datasource {
            if let Some(hikari) = &ds.hikari {
                if hikari.maximum_pool_size.is_none() {
                    issues.push(Issue::new("HIKARI_NO_MAX_POOL", ...));
                }
                if hikari.connection_timeout.is_none() {
                    issues.push(Issue::new("HIKARI_NO_TIMEOUT", ...));
                }
            }
        }
    }

    issues
}
```

**工作量**: ~3-4 小时

---

## 优先级排序

| 任务 | 优先级 | 影响 | 工作量 |
|------|--------|------|--------|
| Call Graph + Import | 高 | N+1 检测准确性 | 2-3h |
| 结构化配置 | 中 | 配置问题检测 | 3-4h |
| Spring Context | 低 | 边界情况 | 1-2h |

---

## 测试策略

每个改进需要:
1. 单元测试覆盖核心逻辑
2. 集成测试验证端到端流程
3. 使用真实 Spring Boot 项目验证

---

*最后更新: 2025-12-27 v9.5.0*
