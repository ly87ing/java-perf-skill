use super::{CodeAnalyzer, Issue, Severity};
use std::path::Path;
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;

/// Dockerfile 分析器
/// 
/// 检测常见的 Dockerfile 性能和安全问题
pub struct DockerfileAnalyzer {
    rules: Vec<DockerfileRule>,
}

struct DockerfileRule {
    id: &'static str,
    severity: Severity,
    pattern: &'static Lazy<Regex>,
    description: &'static str,
}

// 静态编译正则
static RE_FROM_LATEST: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^FROM\s+\S+:latest\b").unwrap()
});

static RE_FROM_NO_TAG: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^FROM\s+(\S+)\s*$").unwrap()
});

static RE_ENV_PASSWORD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^ENV\s+\S*(PASSWORD|SECRET|KEY|TOKEN)\S*\s*=").unwrap()
});

static RE_ADD_REMOTE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^ADD\s+https?://").unwrap()
});

static RE_RUN_APT_NO_CLEAN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^RUN\s+apt(-get)?\s+install").unwrap()
});

impl DockerfileAnalyzer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            rules: vec![
                DockerfileRule {
                    id: "DOCKER_LATEST_TAG",
                    severity: Severity::P0,
                    pattern: &RE_FROM_LATEST,
                    description: "使用 :latest 标签会导致构建不可复现",
                },
                DockerfileRule {
                    id: "DOCKER_NO_TAG",
                    severity: Severity::P0,
                    pattern: &RE_FROM_NO_TAG,
                    description: "FROM 未指定标签，默认使用 :latest",
                },
                DockerfileRule {
                    id: "DOCKER_SENSITIVE_ENV",
                    severity: Severity::P0,
                    pattern: &RE_ENV_PASSWORD,
                    description: "ENV 中包含敏感信息，建议使用 secrets",
                },
                DockerfileRule {
                    id: "DOCKER_ADD_URL",
                    severity: Severity::P1,
                    pattern: &RE_ADD_REMOTE,
                    description: "ADD 远程 URL 不推荐，建议使用 curl + 校验",
                },
            ],
        })
    }
}

impl CodeAnalyzer for DockerfileAnalyzer {
    fn supported_extension(&self) -> &str {
        "Dockerfile"
    }

    fn analyze(&self, code: &str, file_path: &Path) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();
        let file_name = file_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Dockerfile".to_string());

        // 统计 RUN 命令数量
        let mut run_count = 0;
        let mut apt_without_clean = false;

        for (line_num, line) in code.lines().enumerate() {
            let trimmed = line.trim();
            
            // 跳过注释和空行
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }

            // 检查规则
            for rule in &self.rules {
                if rule.pattern.is_match(trimmed) {
                    // 特殊处理 NO_TAG: 排除已有标签的情况
                    if rule.id == "DOCKER_NO_TAG" {
                        // 如果包含 ':' 则说明有标签，跳过
                        if trimmed.contains(':') {
                            continue;
                        }
                    }

                    issues.push(Issue {
                        id: rule.id.to_string(),
                        severity: rule.severity,
                        file: file_name.clone(),
                        line: line_num + 1,
                        description: rule.description.to_string(),
                        context: Some(trimmed.chars().take(60).collect()),
                    });
                }
            }

            // 统计 RUN 命令
            if trimmed.to_uppercase().starts_with("RUN ") {
                run_count += 1;
            }

            // 检查 apt install 是否有 clean
            if RE_RUN_APT_NO_CLEAN.is_match(trimmed) {
                if !code.contains("apt-get clean") && !code.contains("rm -rf /var/lib/apt") {
                    apt_without_clean = true;
                }
            }
        }

        // 检查多个 RUN 命令 (建议合并)
        if run_count > 5 {
            issues.push(Issue {
                id: "DOCKER_MANY_LAYERS".to_string(),
                severity: Severity::P1,
                file: file_name.clone(),
                line: 1,
                description: format!("有 {} 个 RUN 命令，建议使用 && 合并减少层数", run_count),
                context: None,
            });
        }

        // apt install 未清理缓存
        if apt_without_clean {
            issues.push(Issue {
                id: "DOCKER_APT_NO_CLEAN".to_string(),
                severity: Severity::P1,
                file: file_name.clone(),
                line: 1,
                description: "apt-get install 后未清理缓存，镜像体积增大".to_string(),
                context: None,
            });
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_dockerfile_latest_tag() {
        let code = r#"
FROM openjdk:latest
WORKDIR /app
COPY . .
RUN ./gradlew build
        "#;
        
        let analyzer = DockerfileAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &PathBuf::from("Dockerfile")).unwrap();

        assert!(issues.iter().any(|i| i.id == "DOCKER_LATEST_TAG"));
    }

    #[test]
    fn test_dockerfile_no_tag() {
        let code = r#"
FROM ubuntu
RUN apt-get update
        "#;
        
        let analyzer = DockerfileAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &PathBuf::from("Dockerfile")).unwrap();

        assert!(issues.iter().any(|i| i.id == "DOCKER_NO_TAG"));
    }

    #[test]
    fn test_dockerfile_sensitive_env() {
        let code = r#"
FROM node:18
ENV DB_PASSWORD=secret123
RUN npm install
        "#;
        
        let analyzer = DockerfileAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &PathBuf::from("Dockerfile")).unwrap();

        assert!(issues.iter().any(|i| i.id == "DOCKER_SENSITIVE_ENV"));
    }

    #[test]
    fn test_dockerfile_many_layers() {
        let code = r#"
FROM alpine:3.18
RUN apk add curl
RUN apk add bash
RUN apk add git
RUN apk add vim
RUN apk add make
RUN apk add gcc
        "#;
        
        let analyzer = DockerfileAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &PathBuf::from("Dockerfile")).unwrap();

        assert!(issues.iter().any(|i| i.id == "DOCKER_MANY_LAYERS"));
    }
}
