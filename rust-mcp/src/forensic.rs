//! Forensic 模块 - 日志指纹归类分析
//! 
//! 🔬 法医取证：流式处理大日志

use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Duration, Instant};
use regex::Regex;

/// 安全限制
const MAX_MEMORY_MB: usize = 1024;
const MS_PER_MB: u64 = 100;
const MIN_PROCESS_TIME_MS: u64 = 30000;

/// 静态编译的正则表达式
static EXCEPTION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\w+Exception|\w+Error)").unwrap()
});

static LOCATION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\w+\.)+\w+").unwrap()
});

/// 异常指纹
#[derive(Debug, Default)]
struct ExceptionFingerprint {
    exception_type: String,
    location: String,
    count: usize,
    example: String,
}

/// 分析日志文件
pub fn analyze_log(log_path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let path = Path::new(log_path);
    if !path.exists() {
        return Err(format!("Log file not found: {}", log_path).into());
    }
    
    let file = File::open(path)?;
    let file_size = file.metadata()?.len();
    let reader = BufReader::new(file);
    
    // 动态超时
    let file_size_mb = file_size / (1024 * 1024);
    let timeout = Duration::from_millis(
        std::cmp::max(MIN_PROCESS_TIME_MS, file_size_mb * MS_PER_MB)
    );
    
    let start_time = Instant::now();
    let mut exception_map: HashMap<String, ExceptionFingerprint> = HashMap::new();
    let mut lines_processed: usize = 0;
    let mut truncated = false;
    let mut truncate_reason = String::new();
    
    // 流式读取
    for line_result in reader.lines() {
        // 熔断检查：时间
        if start_time.elapsed() > timeout {
            truncated = true;
            truncate_reason = format!(
                "⚠️ 分析超时 (>{}s for {}MB)，已自动终止",
                timeout.as_secs(), file_size_mb
            );
            break;
        }
        
        // 熔断检查：行数（防止内存过大）
        if exception_map.len() > 1000 {
            truncated = true;
            truncate_reason = "⚠️ 异常类型过多 (>1000 种)，已自动终止".to_string();
            break;
        }
        
        if let Ok(line) = line_result {
            lines_processed += 1;
            
            // 提取异常 (使用静态编译的正则)
            if let Some(ex_match) = EXCEPTION_REGEX.find(&line) {
                let ex_type = ex_match.as_str().to_string();
                
                // 提取位置
                let location = LOCATION_REGEX.find(&line)
                    .map(|m| {
                        let parts: Vec<&str> = m.as_str().split('.').collect();
                        if parts.len() >= 2 {
                            format!("{}.{}", parts[parts.len()-2], parts[parts.len()-1])
                        } else {
                            m.as_str().to_string()
                        }
                    })
                    .unwrap_or_else(|| "Unknown".to_string());
                
                let fingerprint = format!("{}@{}", ex_type, location);
                
                let entry = exception_map.entry(fingerprint.clone()).or_insert_with(|| {
                    ExceptionFingerprint {
                        exception_type: ex_type.clone(),
                        location: location.clone(),
                        count: 0,
                        example: line.chars().take(150).collect(),
                    }
                });
                entry.count += 1;
            }
        }
    }
    
    let process_time = start_time.elapsed();
    
    // 排序
    let mut fingerprints: Vec<_> = exception_map.values().collect();
    fingerprints.sort_by(|a, b| b.count.cmp(&a.count));
    
    // 生成报告
    let file_name = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| log_path.to_string());
    
    let mut report = format!(
        "### 日志分析: {}\n\n\
        **性能**: {} 行, {}ms\n",
        file_name,
        lines_processed,
        process_time.as_millis()
    );
    
    if truncated {
        report.push_str(&format!("\n> [!CAUTION]\n> {}\n\n", truncate_reason));
    }
    
    if !fingerprints.is_empty() {
        let total: usize = fingerprints.iter().map(|f| f.count).sum();
        
        report.push_str(&format!(
            "\n## 🔬 异常指纹归类 ({} 类, 共 {} 次)\n\n\
            | # | 类型 | 位置 | 次数 | 标记 |\n\
            |---|------|------|------|------|\n",
            fingerprints.len(), total
        ));
        
        for (i, fp) in fingerprints.iter().take(10).enumerate() {
            let tag = if fp.count > 1000 {
                "🔥 核心噪音"
            } else if fp.count < 10 {
                "⚠️ 可能根因"
            } else if fp.count < 100 {
                "🔍 需关注"
            } else {
                ""
            };
            
            report.push_str(&format!(
                "| {} | `{}` | {} | {} | {} |\n",
                i + 1, fp.exception_type, fp.location, fp.count, tag
            ));
        }
        
        // 关键发现
        let key_errors: Vec<_> = fingerprints.iter().filter(|f| f.count < 10).collect();
        if !key_errors.is_empty() {
            report.push_str(&format!(
                "\n> [!IMPORTANT]\n> 发现 {} 个低频异常，可能是根因！\n",
                key_errors.len()
            ));
        }
    } else {
        report.push_str("\n✅ 未发现异常\n");
    }
    
    Ok(json!(report))
}
