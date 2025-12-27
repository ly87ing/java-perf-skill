use serde::{Serialize, Deserialize};
use std::path::Path;
use anyhow::Result;

pub mod tree_sitter_java;
pub mod config;
pub mod dockerfile;
pub mod rule_handlers;  // v9.2: RuleHandler trait 解耦规则处理

/// 严重级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    P0, // 严重
    P1, // 警告
}

/// 扫描发现的问题
#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub severity: Severity,
    pub file: String,
    pub line: usize,
    pub description: String,
    pub context: Option<String>,
}

/// 代码分析器 Trait
#[allow(dead_code)]
pub trait CodeAnalyzer {
    /// 适用的文件扩展名 (e.g., "java")
    fn supported_extension(&self) -> &str;

    /// 分析代码并返回问题列表
    fn analyze(&self, code: &str, file_path: &Path) -> Result<Vec<Issue>>;
}
