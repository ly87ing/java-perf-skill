//! AST Engine - 高性能正则分析 + 注释过滤
//!
//! 🛰️ 雷达扫描：检测性能反模式
//!
//! 优化点：
//! 1. 使用 once_cell 静态编译正则，避免重复创建
//! 2. 过滤注释内容，避免误报
//! 3. 新增响应式编程问题检测
//! 4. 集成 Tree-sitter AST 分析 (v5.0)
//! 5. 并行文件扫描 (rayon) (v5.1)
//! 6. Dockerfile 扫描 (v5.1)

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Mutex;
use walkdir::WalkDir;
use rayon::prelude::*;

use crate::scanner::{CodeAnalyzer, Issue as ScannerIssue, Severity as ScannerSeverity};
use crate::scanner::tree_sitter_java::JavaTreeSitterAnalyzer;
use crate::scanner::config::LineBasedConfigAnalyzer;
use crate::scanner::dockerfile::DockerfileAnalyzer;

// ============================================================================
// 静态编译正则表达式（只编译一次，全局复用）
// ============================================================================

/// 注释匹配正则（用于过滤）
static COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"//.*$|/\*[\s\S]*?\*/").unwrap()
});

// P0 严重规则
// 注意: N_PLUS_ONE, NESTED_LOOP, SYNC_METHOD, THREADLOCAL 已迁移至 tree_sitter_java.rs 使用 AST 分析
static RE_UNBOUNDED_POOL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"Executors\s*\.\s*(newCachedThreadPool|newScheduledThreadPool|newSingleThreadExecutor)").unwrap()
});
static RE_UNBOUNDED_CACHE_MAP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"static\s+.*Map\s*<[^>]+>\s*\w+\s*=\s*new").unwrap()
});
static RE_UNBOUNDED_CACHE_LIST: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"static\s+.*(List|Set)\s*<[^>]+>\s*\w+\s*=\s*new").unwrap()
});
static RE_EXCEPTION_IGNORE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"catch\s*\([^)]+\)\s*\{\s*\}").unwrap()
});

// P1 警告规则
static RE_OBJECT_IN_LOOP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"for\s*\([^)]+\)\s*\{[^}]*new\s+\w+\s*\(").unwrap()
});
static RE_SYNC_BLOCK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"synchronized\s*\([^)]+\)\s*\{").unwrap()
});
static RE_ATOMIC_SPIN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(AtomicInteger|AtomicLong)\s*[<\s]").unwrap()
});
static RE_NO_TIMEOUT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(HttpClient|RestTemplate|OkHttp|WebClient)\s*\.").unwrap()
});
static RE_BLOCKING_IO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"new\s+File(Input|Output)Stream").unwrap()
});
static RE_STRING_CONCAT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"for\s*\([^)]+\)\s*\{[^}]*\+=").unwrap()
});
static RE_EXCEPTION_SWALLOW: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"catch\s*\([^)]+\)\s*\{[^}]*\.print").unwrap()
});

// 响应式编程问题 (来自 MMS 报告)
static RE_EMITTER_UNBOUNDED: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"EmitterProcessor\s*\.\s*create\s*\(\s*\)").unwrap()
});
static RE_SINKS_NO_BACKPRESSURE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"Sinks\s*\.\s*many\s*\(\s*\)").unwrap()
});

// 缓存配置问题
static RE_CACHE_NO_EXPIRE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(Caffeine|CacheBuilder)\s*\.\s*newBuilder").unwrap()
});

// ============================================================================
// 新增规则 (v5.3)
// ============================================================================

// P0: 阻塞调用无超时
static RE_FUTURE_GET_NO_TIMEOUT: Lazy<Regex> = Lazy::new(|| {
    // 匹配 .get() 但不匹配 .get(timeout, unit)
    Regex::new(r"\.get\s*\(\s*\)").unwrap()
});
static RE_AWAIT_NO_TIMEOUT: Lazy<Regex> = Lazy::new(|| {
    // CountDownLatch.await() 或 Semaphore.acquire() 无超时
    Regex::new(r"\.(await|acquire)\s*\(\s*\)").unwrap()
});
static RE_COMPLETABLE_JOIN: Lazy<Regex> = Lazy::new(|| {
    // CompletableFuture.join() 永久阻塞
    Regex::new(r"\.join\s*\(\s*\)").unwrap()
});

// P0: 锁相关
static RE_REENTRANT_LOCK: Lazy<Regex> = Lazy::new(|| {
    // 检测 ReentrantLock 使用
    Regex::new(r"ReentrantLock|ReadWriteLock|StampedLock").unwrap()
});

// P1: 日志问题
static RE_LOG_STRING_CONCAT: Lazy<Regex> = Lazy::new(|| {
    // logger.debug("x=" + x) 应使用占位符
    Regex::new(r"(log|logger|LOG|LOGGER)\s*\.\s*(debug|info|warn|error|trace)\s*\([^)]*\+").unwrap()
});

// P1: 连接池配置
static RE_DATASOURCE_NO_POOL: Lazy<Regex> = Lazy::new(|| {
    // DriverManager.getConnection 直接使用，无连接池
    Regex::new(r"DriverManager\s*\.\s*getConnection").unwrap()
});

// ============================================================================
// 新增规则 (v7.0) - Spring, 响应式, GC, 数据库
// ============================================================================

// === Spring 相关 ===
static RE_TRANSACTIONAL_REQUIRED_NEW: Lazy<Regex> = Lazy::new(|| {
    // @Transactional(propagation = REQUIRED) 可能导致事务传播问题
    Regex::new(r"@Transactional\s*\(\s*propagation\s*=\s*Propagation\.REQUIRES_NEW").unwrap()
});
static RE_ASYNC_DEFAULT_POOL: Lazy<Regex> = Lazy::new(|| {
    // @Async 未指定线程池，使用默认 SimpleAsyncTaskExecutor
    Regex::new(r"@Async\s*\n\s*public").unwrap()
});
static RE_CACHEABLE_NO_KEY: Lazy<Regex> = Lazy::new(|| {
    // @Cacheable 未指定 key，可能导致缓存冲突
    Regex::new(r"@Cacheable\s*\(\s*[^)]*value\s*=").unwrap()
});
static RE_SCHEDULED_FIXED_RATE: Lazy<Regex> = Lazy::new(|| {
    // @Scheduled(fixedRate) 任务堆积风险
    Regex::new(r"@Scheduled\s*\(\s*fixedRate").unwrap()
});
static RE_AUTOWIRED_FIELD: Lazy<Regex> = Lazy::new(|| {
    // 字段注入不利于测试，建议构造器注入
    Regex::new(r"@Autowired\s*\n\s*private").unwrap()
});

// === 响应式编程 ===
static RE_FLUX_BLOCK: Lazy<Regex> = Lazy::new(|| {
    // Flux/Mono.block() 阻塞调用
    Regex::new(r"\.(block|blockFirst|blockLast)\s*\(").unwrap()
});
static RE_SUBSCRIBE_NO_ERROR: Lazy<Regex> = Lazy::new(|| {
    // subscribe() 未处理 error
    Regex::new(r"\.subscribe\s*\(\s*[^,)]*\s*\)").unwrap()
});
static RE_FLUX_COLLECT_LIST: Lazy<Regex> = Lazy::new(|| {
    // collectList() 可能导致 OOM
    Regex::new(r"\.collectList\s*\(\s*\)").unwrap()
});
static RE_PARALLEL_NO_RUN_ON: Lazy<Regex> = Lazy::new(|| {
    // parallel() 未指定 runOn scheduler
    Regex::new(r"\.parallel\s*\(\s*\)").unwrap()
});

// === GC 相关 ===
static RE_LARGE_ARRAY_ALLOC: Lazy<Regex> = Lazy::new(|| {
    // new byte[1024*1024] 大数组分配
    Regex::new(r"new\s+(byte|char|int|long)\s*\[\s*\d{6,}").unwrap()
});
static RE_FINALIZE_OVERRIDE: Lazy<Regex> = Lazy::new(|| {
    // 重写 finalize() 方法 (已废弃)
    Regex::new(r"protected\s+void\s+finalize\s*\(").unwrap()
});
static RE_SOFT_REFERENCE: Lazy<Regex> = Lazy::new(|| {
    // SoftReference 滥用
    Regex::new(r"new\s+SoftReference\s*<").unwrap()
});
static RE_INTERN_STRING: Lazy<Regex> = Lazy::new(|| {
    // String.intern() 可能导致永久代/元空间溢出
    Regex::new(r"\.intern\s*\(\s*\)").unwrap()
});

// === 数据库 ===
static RE_SELECT_STAR: Lazy<Regex> = Lazy::new(|| {
    // SELECT * 查询
    Regex::new(r#"["']SELECT\s+\*\s+FROM"#).unwrap()
});
static RE_LIKE_LEADING_WILDCARD: Lazy<Regex> = Lazy::new(|| {
    // LIKE '%xxx' 前导通配符导致全表扫描
    Regex::new(r#"LIKE\s+['"]%"#).unwrap()
});

// ============================================================================
// 规则定义
// ============================================================================

/// 问题严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    P0, // 严重
    P1, // 警告
}

/// AST 检测问题
#[derive(Debug)]
pub struct AstIssue {
    pub severity: Severity,
    pub issue_type: String,
    pub file: String,
    pub line: usize,
    pub description: String,
}

/// 规则配置
struct Rule {
    id: &'static str,
    description: &'static str,
    severity: Severity,
    regex: &'static Lazy<Regex>,
}

/// 所有规则（引用静态编译的正则）
fn get_rules() -> Vec<Rule> {
    vec![
        // AST Migrated Rules (Commented out / handled by Tree-sitter)
        // Rule { id: "N_PLUS_ONE", ... }
        // Rule { id: "NESTED_LOOP", ... }
        // Rule { id: "SYNC_METHOD", ... }
        
        // P0 严重 - 原有规则
        Rule { id: "UNBOUNDED_POOL", description: "无界线程池 Executors", severity: Severity::P0, regex: &RE_UNBOUNDED_POOL },
        Rule { id: "UNBOUNDED_CACHE", description: "无界缓存 static Map", severity: Severity::P0, regex: &RE_UNBOUNDED_CACHE_MAP },
        Rule { id: "UNBOUNDED_LIST", description: "无界缓存 static List/Set", severity: Severity::P0, regex: &RE_UNBOUNDED_CACHE_LIST },
        Rule { id: "EXCEPTION_IGNORE", description: "空 catch 块", severity: Severity::P0, regex: &RE_EXCEPTION_IGNORE },
        Rule { id: "EMITTER_UNBOUNDED", description: "EmitterProcessor 无界 (背压问题)", severity: Severity::P0, regex: &RE_EMITTER_UNBOUNDED },
        
        // P0 严重 - 新增规则 (v5.3)
        Rule { id: "FUTURE_GET_NO_TIMEOUT", description: "Future.get() 无超时，可能永久阻塞", severity: Severity::P0, regex: &RE_FUTURE_GET_NO_TIMEOUT },
        Rule { id: "AWAIT_NO_TIMEOUT", description: "await()/acquire() 无超时，可能永久阻塞", severity: Severity::P0, regex: &RE_AWAIT_NO_TIMEOUT },
        Rule { id: "REENTRANT_LOCK_RISK", description: "ReentrantLock 使用 (确保 unlock 在 finally)", severity: Severity::P0, regex: &RE_REENTRANT_LOCK },
        
        // P1 警告 - 原有规则
        Rule { id: "OBJECT_IN_LOOP", description: "循环内创建对象", severity: Severity::P1, regex: &RE_OBJECT_IN_LOOP },
        Rule { id: "SYNC_BLOCK", description: "synchronized 代码块", severity: Severity::P1, regex: &RE_SYNC_BLOCK },
        Rule { id: "ATOMIC_SPIN", description: "Atomic 自旋 (考虑 LongAdder)", severity: Severity::P1, regex: &RE_ATOMIC_SPIN },
        Rule { id: "NO_TIMEOUT", description: "HTTP 客户端可能无超时", severity: Severity::P1, regex: &RE_NO_TIMEOUT },
        Rule { id: "BLOCKING_IO", description: "同步文件 IO", severity: Severity::P1, regex: &RE_BLOCKING_IO },
        Rule { id: "STRING_CONCAT", description: "循环内字符串拼接", severity: Severity::P1, regex: &RE_STRING_CONCAT },
        Rule { id: "EXCEPTION_SWALLOW", description: "异常被吞没 (仅打印)", severity: Severity::P1, regex: &RE_EXCEPTION_SWALLOW },
        Rule { id: "SINKS_NO_BACKPRESSURE", description: "Sinks.many() 无背压处理", severity: Severity::P1, regex: &RE_SINKS_NO_BACKPRESSURE },
        Rule { id: "CACHE_NO_EXPIRE", description: "Cache 可能无过期配置", severity: Severity::P1, regex: &RE_CACHE_NO_EXPIRE },
        
        // P1 警告 - 新增规则 (v5.3)
        Rule { id: "COMPLETABLE_JOIN", description: "CompletableFuture.join() 无超时", severity: Severity::P1, regex: &RE_COMPLETABLE_JOIN },
        Rule { id: "LOG_STRING_CONCAT", description: "日志字符串拼接 (应用占位符)", severity: Severity::P1, regex: &RE_LOG_STRING_CONCAT },
        Rule { id: "DATASOURCE_NO_POOL", description: "DriverManager 直接获取连接 (无连接池)", severity: Severity::P1, regex: &RE_DATASOURCE_NO_POOL },
        
        // ====== v7.0 新增规则 ======
        
        // Spring 相关 (P1)
        Rule { id: "TRANSACTIONAL_REQUIRES_NEW", description: "@Transactional(REQUIRES_NEW) 事务嵌套风险", severity: Severity::P1, regex: &RE_TRANSACTIONAL_REQUIRED_NEW },
        Rule { id: "ASYNC_DEFAULT_POOL", description: "@Async 未指定线程池，使用默认 SimpleAsyncTaskExecutor", severity: Severity::P1, regex: &RE_ASYNC_DEFAULT_POOL },
        Rule { id: "CACHEABLE_NO_KEY", description: "@Cacheable 未指定 key，可能导致缓存冲突", severity: Severity::P1, regex: &RE_CACHEABLE_NO_KEY },
        Rule { id: "SCHEDULED_FIXED_RATE", description: "@Scheduled(fixedRate) 任务堆积风险", severity: Severity::P1, regex: &RE_SCHEDULED_FIXED_RATE },
        Rule { id: "AUTOWIRED_FIELD", description: "字段注入不利于测试，建议构造器注入", severity: Severity::P1, regex: &RE_AUTOWIRED_FIELD },
        
        // 响应式编程 (P0/P1)
        Rule { id: "FLUX_BLOCK", description: "Flux/Mono.block() 阻塞调用，可能死锁", severity: Severity::P0, regex: &RE_FLUX_BLOCK },
        Rule { id: "SUBSCRIBE_NO_ERROR", description: "subscribe() 未处理 error，异常会被吞没", severity: Severity::P1, regex: &RE_SUBSCRIBE_NO_ERROR },
        Rule { id: "FLUX_COLLECT_LIST", description: "collectList() 可能导致 OOM", severity: Severity::P1, regex: &RE_FLUX_COLLECT_LIST },
        Rule { id: "PARALLEL_NO_RUN_ON", description: "parallel() 未指定 runOn scheduler", severity: Severity::P1, regex: &RE_PARALLEL_NO_RUN_ON },
        
        // GC 相关 (P1)
        Rule { id: "LARGE_ARRAY_ALLOC", description: "大数组分配，可能触发 Full GC", severity: Severity::P1, regex: &RE_LARGE_ARRAY_ALLOC },
        Rule { id: "FINALIZE_OVERRIDE", description: "重写 finalize() 方法 (已废弃，影响 GC)", severity: Severity::P0, regex: &RE_FINALIZE_OVERRIDE },
        Rule { id: "SOFT_REFERENCE_MISUSE", description: "SoftReference 滥用可能导致内存问题", severity: Severity::P1, regex: &RE_SOFT_REFERENCE },
        Rule { id: "STRING_INTERN", description: "String.intern() 可能导致元空间溢出", severity: Severity::P1, regex: &RE_INTERN_STRING },
        
        // 数据库 (P1)
        Rule { id: "SELECT_STAR", description: "SELECT * 查询，建议明确指定字段", severity: Severity::P1, regex: &RE_SELECT_STAR },
        Rule { id: "LIKE_LEADING_WILDCARD", description: "LIKE '%xxx' 前导通配符导致全表扫描", severity: Severity::P0, regex: &RE_LIKE_LEADING_WILDCARD },
    ]
}

// Helper to convert ScannerIssue to AstIssue
fn convert_issue(issue: ScannerIssue) -> AstIssue {
    let sev = match issue.severity {
        ScannerSeverity::P0 => Severity::P0,
        ScannerSeverity::P1 => Severity::P1,
    };
    AstIssue {
        severity: sev,
        issue_type: issue.id,
        file: issue.file,
        line: issue.line,
        description: issue.description,
    }
}

// ============================================================================
// 核心扫描函数
// ============================================================================

/// 全项目雷达扫描 (v5.1 并行版本)
/// 
/// compact: true 时只返回 P0，每个 issue 只有 id/file/line
/// max_p1: compact=false 时最多返回的 P1 数量
pub fn radar_scan(code_path: &str, compact: bool, max_p1: usize) -> Result<Value, Box<dyn std::error::Error>> {
    let path = Path::new(code_path);
    
    // 收集所有待扫描文件
    let entries: Vec<_> = WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    let file_count = entries.len();

    // 使用 Mutex 保护共享状态 (rayon 并行安全)
    let issues: Mutex<Vec<AstIssue>> = Mutex::new(Vec::new());

    // 预初始化分析器 (在并行前创建，每个线程克隆使用或按需创建)
    // 注意：由于 Tree-sitter 的 Query 不是 Send，我们在每个线程内创建分析器

    // 并行处理文件
    entries.par_iter().for_each(|entry| {
        let file_path = entry.path();
        let file_name_str = file_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // 本线程的 issues
        let mut local_issues: Vec<AstIssue> = Vec::new();

        if ext == "java" {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                // 1. Regex Analysis (Legacy)
                let legacy = analyze_java_code(&content, &file_path.to_string_lossy());
                local_issues.extend(legacy);

                // 2. AST Analysis
                if let Ok(analyzer) = JavaTreeSitterAnalyzer::new() {
                    if let Ok(ast_results) = analyzer.analyze(&content, file_path) {
                        local_issues.extend(ast_results.into_iter().map(convert_issue));
                    }
                }
            }
        } else if ["yml", "yaml", "properties"].contains(&ext) {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                // 3. Config Analysis
                if let Ok(analyzer) = LineBasedConfigAnalyzer::new() {
                    if let Ok(config_results) = analyzer.analyze(&content, file_path) {
                        local_issues.extend(config_results.into_iter().map(convert_issue));
                    }
                }
            }
        } else if file_name_str == "Dockerfile" || file_name_str.starts_with("Dockerfile.") {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                // 4. Dockerfile Analysis (v5.1 NEW)
                if let Ok(analyzer) = DockerfileAnalyzer::new() {
                    if let Ok(docker_results) = analyzer.analyze(&content, file_path) {
                        local_issues.extend(docker_results.into_iter().map(convert_issue));
                    }
                }
            }
        }

        // 合并到全局 issues
        if !local_issues.is_empty() {
            // 使用 unwrap_or_else 处理 poisoned mutex（如果持锁线程 panic）
            let mut global = issues.lock().unwrap_or_else(|e| e.into_inner());
            global.extend(local_issues);
        }
    });

    // 安全地解包：如果 mutex 被 poisoned，仍然获取内部数据
    let issues = issues.into_inner().unwrap_or_else(|e| e.into_inner());
    let p0_count = issues.iter().filter(|i| matches!(i.severity, Severity::P0)).count();
    let p1_count = issues.iter().filter(|i| matches!(i.severity, Severity::P1)).count();

    // === 根据 compact 模式生成不同报告 ===
    if compact {
        // 紧凑模式：只返回 P0，精简格式
        let mut report = format!(
            "## 🛰️ 雷达扫描 (v5.1 并行)\n\n**P0**: {p0_count} | **P1**: {p1_count} | **文件**: {file_count}\n\n"
        );

        if p0_count > 0 {
            for issue in issues.iter().filter(|i| matches!(i.severity, Severity::P0)) {
                report.push_str(&format!(
                    "- `{}` {}:{}\n",
                    issue.issue_type, issue.file, issue.line
                ));
            }
        } else {
            report.push_str("✅ 无 P0 问题\n");
        }

        if p1_count > 0 {
            report.push_str(&format!("\n*（{p1_count} 个 P1 警告已省略，使用 compact=false 查看）*\n"));
        }

        Ok(json!(report))
    } else {
        // 完整模式
        let mut report = format!(
            "## 🛰️ 雷达扫描结果 (v5.1 并行 + Dockerfile)\n\n\
            **扫描**: {} 个文件\n\
            **发现**: {} 个嫌疑点 (P0: {}, P1: {})\n\n",
            file_count, issues.len(), p0_count, p1_count
        );

        if p0_count > 0 {
            report.push_str("### 🔴 P0 严重嫌疑\n\n");
            for issue in issues.iter().filter(|i| matches!(i.severity, Severity::P0)) {
                report.push_str(&format!(
                    "- **{}** - `{}:{}` - {}\n",
                    issue.issue_type, issue.file, issue.line, issue.description
                ));
            }
            report.push('\n');
        }

        if p1_count > 0 {
            report.push_str(&format!("### 🟡 P1 警告 (显示前 {max_p1})\n\n"));
            for issue in issues.iter().filter(|i| matches!(i.severity, Severity::P1)).take(max_p1) {
                report.push_str(&format!(
                    "- **{}** - `{}:{}` - {}\n",
                    issue.issue_type, issue.file, issue.line, issue.description
                ));
            }
        }

        Ok(json!(report))
    }
}

/// 单文件扫描
pub fn scan_source_code(code: &str, file_path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut issues = Vec::new();
    let path = Path::new(file_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if ext == "java" {
        // Regex
        issues.extend(analyze_java_code(code, file_path));
        // AST
        if let Ok(analyzer) = JavaTreeSitterAnalyzer::new() {
             if let Ok(res) = analyzer.analyze(code, path) {
                 issues.extend(res.into_iter().map(convert_issue));
             }
        }
    } else if ["yml", "yaml", "properties"].contains(&ext) {
        // Config
        if let Ok(analyzer) = LineBasedConfigAnalyzer::new() {
             if let Ok(res) = analyzer.analyze(code, path) {
                 issues.extend(res.into_iter().map(convert_issue));
             }
        }
    }

    let mut report = format!("## 🛰️ 扫描: {file_path}\n\n");

    if issues.is_empty() {
        report.push_str("✅ 未发现明显性能问题\n");
    } else {
        for issue in &issues {
            let emoji = match issue.severity {
                Severity::P0 => "🔴",
                Severity::P1 => "🟡",
            };
            report.push_str(&format!(
                "{} **{}** (行 {}) - {}\n",
                emoji, issue.issue_type, issue.line, issue.description
            ));
        }
    }

    Ok(json!(report))
}

/// 分析 Java 代码（高性能版本 - Legacy Regex）
fn analyze_java_code(code: &str, file_path: &str) -> Vec<AstIssue> {
    let mut issues = Vec::new();
    let file_name = Path::new(file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());

    // 1. 移除注释，避免误报
    let code_without_comments = COMMENT_REGEX.replace_all(code, "");

    // 2. 特殊检测：ThreadLocal (MIGRATED TO AST -> DISABLED HERE)
    /*
    if RE_THREADLOCAL.is_match(&code_without_comments) {
        if !code_without_comments.contains(".remove()") {
            if let Some(mat) = RE_THREADLOCAL.find(&code_without_comments) {
                let line_num = code_without_comments[..mat.start()].matches('\n').count() + 1;
                issues.push(AstIssue {
                    severity: Severity::P0,
                    issue_type: "THREADLOCAL_LEAK".to_string(),
                    file: file_name.clone(),
                    line: line_num,
                    description: "ThreadLocal 未调用 remove()，线程池复用会导致内存泄露".to_string(),
                });
            }
        }
    }
    */

    // 3. 特殊检测：Cache 需要 expire 配置
    if RE_CACHE_NO_EXPIRE.is_match(&code_without_comments)
        && !code_without_comments.contains("expire") && !code_without_comments.contains("maximumSize") {
            if let Some(mat) = RE_CACHE_NO_EXPIRE.find(&code_without_comments) {
                let line_num = code_without_comments[..mat.start()].matches('\n').count() + 1;
                issues.push(AstIssue {
                    severity: Severity::P1,
                    issue_type: "CACHE_NO_EXPIRE".to_string(),
                    file: file_name.clone(),
                    line: line_num,
                    description: "Caffeine/Guava Cache 未设置 expire 或 maximumSize".to_string(),
                });
            }
        }

    // 4. 使用静态编译的正则进行匹配
    let rules = get_rules();
    for rule in &rules {
        // 跳过已特殊处理的规则
        if rule.id == "CACHE_NO_EXPIRE" {
            continue;
        }

        if rule.regex.is_match(&code_without_comments) {
            if let Some(mat) = rule.regex.find(&code_without_comments) {
                let line_num = code_without_comments[..mat.start()].matches('\n').count() + 1;

                // 去重
                let exists = issues.iter().any(|i| i.issue_type == rule.id && i.line == line_num);

                if !exists {
                    issues.push(AstIssue {
                        severity: rule.severity,
                        issue_type: rule.id.to_string(),
                        file: file_name.clone(),
                        line: line_num,
                        description: rule.description.to_string(),
                    });
                }
            }
        }
    }

    issues
}
