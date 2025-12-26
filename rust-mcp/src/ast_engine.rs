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
static RE_N_PLUS_ONE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)for\s*\([^)]+\)\s*\{[^}]*(dao|repository|mapper|jdbc|select|insert|update|delete|http|client)[^}]*\}").unwrap()
});
static RE_NESTED_LOOP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"for\s*\([^)]+\)\s*\{[^}]*for\s*\([^)]+\)").unwrap()
});
static RE_SYNC_METHOD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"public\s+synchronized\s+\w+\s+\w+\s*\(").unwrap()
});
static RE_THREADLOCAL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"ThreadLocal\s*<").unwrap()
});
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
        
        // P0 严重
        Rule { id: "UNBOUNDED_POOL", description: "无界线程池 Executors", severity: Severity::P0, regex: &RE_UNBOUNDED_POOL },
        Rule { id: "UNBOUNDED_CACHE", description: "无界缓存 static Map", severity: Severity::P0, regex: &RE_UNBOUNDED_CACHE_MAP },
        Rule { id: "UNBOUNDED_LIST", description: "无界缓存 static List/Set", severity: Severity::P0, regex: &RE_UNBOUNDED_CACHE_LIST },
        Rule { id: "EXCEPTION_IGNORE", description: "空 catch 块", severity: Severity::P0, regex: &RE_EXCEPTION_IGNORE },
        Rule { id: "EMITTER_UNBOUNDED", description: "EmitterProcessor 无界 (背压问题)", severity: Severity::P0, regex: &RE_EMITTER_UNBOUNDED },
        // P1 警告
        Rule { id: "OBJECT_IN_LOOP", description: "循环内创建对象", severity: Severity::P1, regex: &RE_OBJECT_IN_LOOP },
        Rule { id: "SYNC_BLOCK", description: "synchronized 代码块", severity: Severity::P1, regex: &RE_SYNC_BLOCK },
        Rule { id: "ATOMIC_SPIN", description: "Atomic 自旋 (考虑 LongAdder)", severity: Severity::P1, regex: &RE_ATOMIC_SPIN },
        Rule { id: "NO_TIMEOUT", description: "HTTP 客户端可能无超时", severity: Severity::P1, regex: &RE_NO_TIMEOUT },
        Rule { id: "BLOCKING_IO", description: "同步文件 IO", severity: Severity::P1, regex: &RE_BLOCKING_IO },
        Rule { id: "STRING_CONCAT", description: "循环内字符串拼接", severity: Severity::P1, regex: &RE_STRING_CONCAT },
        Rule { id: "EXCEPTION_SWALLOW", description: "异常被吞没 (仅打印)", severity: Severity::P1, regex: &RE_EXCEPTION_SWALLOW },
        Rule { id: "SINKS_NO_BACKPRESSURE", description: "Sinks.many() 无背压处理", severity: Severity::P1, regex: &RE_SINKS_NO_BACKPRESSURE },
        Rule { id: "CACHE_NO_EXPIRE", description: "Cache 可能无过期配置", severity: Severity::P1, regex: &RE_CACHE_NO_EXPIRE },
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
            let mut global = issues.lock().unwrap();
            global.extend(local_issues);
        }
    });

    let issues = issues.into_inner().unwrap();
    let p0_count = issues.iter().filter(|i| matches!(i.severity, Severity::P0)).count();
    let p1_count = issues.iter().filter(|i| matches!(i.severity, Severity::P1)).count();

    // === 根据 compact 模式生成不同报告 ===
    if compact {
        // 紧凑模式：只返回 P0，精简格式
        let mut report = format!(
            "## 🛰️ 雷达扫描 (v5.1 并行)\n\n**P0**: {} | **P1**: {} | **文件**: {}\n\n",
            p0_count, p1_count, file_count
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
            report.push_str(&format!("\n*（{} 个 P1 警告已省略，使用 compact=false 查看）*\n", p1_count));
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
            report.push_str(&format!("### 🟡 P1 警告 (显示前 {})\n\n", max_p1));
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

    let mut report = format!("## 🛰️ 扫描: {}\n\n", file_path);

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
    if RE_CACHE_NO_EXPIRE.is_match(&code_without_comments) {
        if !code_without_comments.contains("expire") && !code_without_comments.contains("maximumSize") {
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
