use serde::{Serialize, Deserialize};
use std::path::Path;
use anyhow::Result;

pub mod tree_sitter_java;
pub mod config;
pub mod dockerfile;
pub mod rule_handlers;  // v9.2: RuleHandler trait 解耦规则处理
pub mod queries;        // v9.4: 外部化 Query 加载

/// 严重级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    P0, // 严重
    P1, // 警告
}

/// Confidence level for issue detection
/// 
/// Used to indicate how confident the analyzer is about a detected issue.
/// High confidence means the issue was detected using semantic analysis (FQN resolution),
/// while Low confidence means heuristic fallback was used.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Confidence {
    /// High confidence - FQN was resolved successfully
    High,
    /// Medium confidence - partial resolution or strong heuristic match
    Medium,
    /// Low confidence - heuristic fallback was used
    Low,
}

/// 扫描发现的问题
#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub severity: Severity,
    pub file: String,
    pub line: usize,
    /// 精确列位置，用于 JetBrains MCP get_symbol_at_location 调用
    #[serde(default)]
    pub column: usize,
    pub description: String,
    pub context: Option<String>,
    /// Confidence level for this issue detection
    /// 
    /// - `Some(High)`: FQN was resolved successfully (semantic analysis)
    /// - `Some(Medium)`: Partial resolution or strong heuristic match
    /// - `Some(Low)`: Heuristic fallback was used
    /// - `None`: Confidence not applicable for this rule type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
}

/// 代码分析器 Trait
#[allow(dead_code)]
pub trait CodeAnalyzer {
    /// 适用的文件扩展名 (e.g., "java")
    fn supported_extension(&self) -> &str;

    /// 分析代码并返回问题列表
    fn analyze(&self, code: &str, file_path: &Path) -> Result<Vec<Issue>>;
}
