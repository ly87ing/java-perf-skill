use super::{CodeAnalyzer, Issue, Severity};
use std::path::Path;
use anyhow::Result;

/// 基于行的配置分析器
/// 
/// 暂时不引入重量级的 YAML parser，而是使用行匹配 (Line-based Matching)
/// 这足以处理 key=value (properties) 和 key: value (yaml) 的简单情况
pub struct LineBasedConfigAnalyzer {
    rules: Vec<ConfigRule>,
}

struct ConfigRule {
    id: &'static str,
    severity: Severity,
    // 完整 Key (用于 Properties)
    full_key: &'static str,
    // 简单 Key (用于 YAML 行匹配，如 "max-threads")
    simple_key: &'static str,
    validator: fn(&str) -> bool,
    description: &'static str,
}

impl LineBasedConfigAnalyzer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            rules: vec![
                ConfigRule {
                    id: "DB_POOL_SMALL",
                    severity: Severity::P1,
                    full_key: "spring.datasource.hikari.maximum-pool-size",
                    simple_key: "maximum-pool-size",
                    validator: |val| {
                        let v = val.split('#').next().unwrap_or("").trim();
                        if let Ok(num) = v.parse::<i32>() {
                            return num >= 5;
                        }
                        true
                    },
                    description: "数据库连接池过小 (建议 >= 10)",
                },
                ConfigRule {
                    id: "TOMCAT_THREADS_LOW",
                    severity: Severity::P1,
                    full_key: "server.tomcat.max-threads",
                    simple_key: "max-threads",
                    validator: |val| {
                        let v = val.split('#').next().unwrap_or("").trim();
                        if let Ok(num) = v.parse::<i32>() {
                            return num >= 200;
                        }
                        true
                    },
                    description: "Tomcat 最大线程数过低 (默认 200)",
                },
            ],
        })
    }
}

impl CodeAnalyzer for LineBasedConfigAnalyzer {
    fn supported_extension(&self) -> &str {
        "properties|yml|yaml"
    }

    fn analyze(&self, code: &str, file_path: &Path) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();
        let file_name = file_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "config".to_string());

        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !["properties", "yml", "yaml"].contains(&ext) {
             return Ok(vec![]);
        }
        
        // 简单判断是否是 YAML (通过扩展名)
        let is_yaml = ["yml", "yaml"].contains(&ext);

        for (line_num, line) in code.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }

            for rule in &self.rules {
                // 根据文件类型选择匹配模式
                let pattern = if is_yaml { rule.simple_key } else { rule.full_key };
                
                // 检查是否包含 key
                if trimmed.contains(pattern) {
                    let parts: Vec<&str> = if trimmed.contains('=') {
                        trimmed.splitn(2, '=').collect()
                    } else {
                        trimmed.splitn(2, ':').collect()
                    };

                    if parts.len() == 2 {
                        let key_part = parts[0].trim();
                        let value_part = parts[1].trim();

                        // 确保 key 匹配 (Key 必须以 pattern 结尾)
                        if key_part.ends_with(pattern) {
                             if !(rule.validator)(value_part) {
                                 issues.push(Issue {
                                    id: rule.id.to_string(),
                                    severity: rule.severity,
                                    file: file_name.clone(),
                                    line: line_num + 1,
                                    description: format!("{} (Value: {})", rule.description, value_part),
                                    context: Some(line.to_string()),
                                });
                             }
                        }
                    }
                }
            }
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_yaml_config() {
        let code = r#"
spring:
  datasource:
    hikari:
      maximum-pool-size: 2  # Too small!
      minimum-idle: 1
server:
  tomcat:
    max-threads: 50 # Too small!
        "#;
        
        let analyzer = LineBasedConfigAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &PathBuf::from("application.yml")).unwrap();

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].id, "DB_POOL_SMALL");
        assert_eq!(issues[1].id, "TOMCAT_THREADS_LOW");
    }

    #[test]
    fn test_properties_config() {
        let code = r#"
spring.datasource.hikari.maximum-pool-size=3
server.tomcat.max-threads=250
        "#;
        
        let analyzer = LineBasedConfigAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &PathBuf::from("application.properties")).unwrap();

        // only pool size is small
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "DB_POOL_SMALL");
    }
}
