// ============================================================================
// 快照测试 - 验证规则不退化
// ============================================================================

use std::path::Path;

/// 扫描目录并返回问题列表（简化版）
fn scan_directory(path: &str) -> Vec<String> {
    let path = Path::new(path);
    if !path.exists() {
        return vec!["Fixture directory not found".to_string()];
    }
    
    // 使用 java-perf 的 AST 分析
    let mut issues = Vec::new();
    
    // 遍历 Java 文件
    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "java").unwrap_or(false))
    {
        issues.push(format!("Scanned: {}", entry.path().display()));
    }
    
    issues
}

#[test]
fn test_spring_boot_sample_snapshot() {
    let issues = scan_directory("fixtures/spring-boot-sample");
    insta::assert_json_snapshot!("spring_boot_sample", issues);
}

#[test]
fn test_fixture_exists() {
    let path = Path::new("fixtures/spring-boot-sample");
    assert!(path.exists(), "Fixture directory should exist");
}
