use super::{CodeAnalyzer, Issue, Severity};
use std::path::Path;
use anyhow::{Result, anyhow};
use tree_sitter::{Parser, Query, QueryCursor};

/// 预编译的规则
struct CompiledRule {
    id: &'static str,
    severity: Severity,
    query: Query,
    description: &'static str,
}

pub struct JavaTreeSitterAnalyzer {
    language: tree_sitter::Language,
    /// 预编译的查询 (在 new() 时编译一次)
    compiled_rules: Vec<CompiledRule>,
}

impl JavaTreeSitterAnalyzer {
    pub fn new() -> Result<Self> {
        let language = tree_sitter_java::language();
        
        // 预编译所有查询
        let compiled_rules = Self::compile_rules(&language)?;
        
        Ok(Self {
            language,
            compiled_rules,
        })
    }

    /// 编译规则查询 (只在初始化时调用一次)
    fn compile_rules(language: &tree_sitter::Language) -> Result<Vec<CompiledRule>> {
        let rule_defs = vec![
            // 规则1: N_PLUS_ONE (循环内的 Repository 调用)
            ("N_PLUS_ONE", Severity::P0, r#"
                (for_statement
                    body: (block
                        (expression_statement
                            (method_invocation
                                name: (identifier) @method_name
                            ) @call
                        )
                    )
                )
            "#, "循环内调用方法 (可能是 N+1 问题)"),
            
            // 规则2: NESTED_LOOP (嵌套循环)
            ("NESTED_LOOP", Severity::P0, r#"
                (for_statement
                    body: (block
                        (for_statement) @inner_loop
                    )
                )
            "#, "嵌套循环检测 (可能导致 O(N^2) 复杂度)"),
            
            // 规则3: SYNC_METHOD (方法级同步)
            ("SYNC_METHOD", Severity::P0, r#"
                (method_declaration
                    (modifiers) @mods
                )
            "#, "Synchronized 方法级锁 (建议改用细粒度锁)"),
            
            // 规则4: THREADLOCAL_LEAK (P0)
            ("THREADLOCAL_LEAK", Severity::P0, r#"
                (method_invocation
                    object: (identifier) @var_name
                    name: (identifier) @method
                    (#eq? @method "set")
                ) @set_call
            "#, "ThreadLocal.set() 后未在同一方法内调用 remove()"),
        ];

        let mut compiled = Vec::with_capacity(rule_defs.len());
        
        for (id, severity, query_str, description) in rule_defs {
            let query = Query::new(language, query_str)
                .map_err(|e| anyhow!("Failed to compile query for {}: {}", id, e))?;
            
            compiled.push(CompiledRule {
                id,
                severity,
                query,
                description,
            });
        }
        
        Ok(compiled)
    }
}

impl CodeAnalyzer for JavaTreeSitterAnalyzer {
    fn supported_extension(&self) -> &str {
        "java"
    }

    fn analyze(&self, code: &str, file_path: &Path) -> Result<Vec<Issue>> {
        let mut parser = Parser::new();
        parser.set_language(&self.language).map_err(|e| anyhow!("Failed to set language: {}", e))?;

        let tree = parser.parse(code, None).ok_or_else(|| anyhow!("Failed to parse code"))?;
        let root_node = tree.root_node();
        let mut issues = Vec::new();

        // 使用预编译的查询 (不再每次编译)
        for rule in &self.compiled_rules {
            let mut query_cursor = QueryCursor::new();
            let matches = query_cursor.matches(&rule.query, root_node, code.as_bytes());

            for m in matches {
                match rule.id {
                    "N_PLUS_ONE" => {
                        let method_name_idx = rule.query.capture_index_for_name("method_name").unwrap();
                        let call_idx = rule.query.capture_index_for_name("call").unwrap();
                        let mut method_name_text = String::new();
                        let mut line = 0;
                        
                        for capture in m.captures {
                            if capture.index == method_name_idx {
                                method_name_text = capture.node.utf8_text(code.as_bytes()).unwrap_or("").to_string();
                            }
                            if capture.index == call_idx {
                                line = capture.node.start_position().row + 1;
                            }
                        }

                        if method_name_text.contains("find") || 
                           method_name_text.contains("save") || 
                           method_name_text.contains("select") || 
                           method_name_text.contains("delete") {
                            
                            let file_name = file_path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "unknown".to_string());

                            issues.push(Issue {
                                id: rule.id.to_string(),
                                severity: rule.severity,
                                file: file_name,
                                line,
                                description: format!("{} (Method: {})", rule.description, method_name_text),
                                context: Some(method_name_text),
                            });
                        }
                    },
                    "NESTED_LOOP" => {
                        let inner_loop_idx = rule.query.capture_index_for_name("inner_loop").unwrap();
                        for capture in m.captures {
                            if capture.index == inner_loop_idx {
                                let line = capture.node.start_position().row + 1;
                                issues.push(Issue {
                                    id: rule.id.to_string(),
                                    severity: rule.severity,
                                    file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                    line,
                                    description: rule.description.to_string(),
                                    context: None,
                                });
                            }
                        }
                    },
                    "SYNC_METHOD" => {
                        let mods_idx = rule.query.capture_index_for_name("mods").unwrap();
                        for capture in m.captures {
                            if capture.index == mods_idx {
                                let mods_text = capture.node.utf8_text(code.as_bytes()).unwrap_or("");
                                if mods_text.contains("synchronized") {
                                    let line = capture.node.start_position().row + 1;
                                    issues.push(Issue {
                                        id: rule.id.to_string(),
                                        severity: rule.severity,
                                        file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                        line,
                                        description: rule.description.to_string(),
                                        context: Some(mods_text.to_string()),
                                    });
                                }
                            }
                        }
                    },
                    "THREADLOCAL_LEAK" => {
                        let set_call_idx = rule.query.capture_index_for_name("set_call").unwrap();
                        let var_name_idx = rule.query.capture_index_for_name("var_name").unwrap();
                        
                        let mut var_name = String::new();
                        let mut set_node = None;

                        for capture in m.captures {
                            if capture.index == var_name_idx {
                                var_name = capture.node.utf8_text(code.as_bytes()).unwrap_or("").to_string();
                            }
                            if capture.index == set_call_idx {
                                set_node = Some(capture.node);
                            }
                        }

                        if !var_name.is_empty() && set_node.is_some() {
                            let node = set_node.unwrap();
                            // 向上查找 method_declaration
                            let mut current = node.parent();
                            let mut method_node = None;
                            
                            while let Some(n) = current {
                                if n.kind() == "method_declaration" {
                                    method_node = Some(n);
                                    break;
                                }
                                current = n.parent();
                            }

                            if let Some(method) = method_node {
                                let method_text = method.utf8_text(code.as_bytes()).unwrap_or("");
                                let remove_call = format!("{}.remove()", var_name);
                                
                                if !method_text.contains(&remove_call) {
                                     let line = node.start_position().row + 1;
                                     issues.push(Issue {
                                        id: rule.id.to_string(),
                                        severity: rule.severity,
                                        file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                        line,
                                        description: format!("{} (Variable: {})", rule.description, var_name),
                                        context: Some(var_name),
                                    });
                                }
                            }
                        }
                    },
                    _ => {}
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
    fn test_n_plus_one_detection() {
        let code = r#"
            public class Test {
                public void process() {
                    for (int i = 0; i < 10; i++) {
                        repository.save(i);
                        userDao.findById(i);
                        System.out.println(i);
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].id, "N_PLUS_ONE");
        assert!(issues[0].context.as_ref().unwrap().contains("save"));
        
        assert_eq!(issues[1].id, "N_PLUS_ONE");
        assert!(issues[1].context.as_ref().unwrap().contains("findById"));
    }

    #[test]
    fn test_nested_loop_detection() {
        let code = r#"
            public class Test {
                public void process() {
                    for (int i = 0; i < 10; i++) {
                        for (int j = 0; j < 10; j++) {
                            // nested loop
                        }
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "NESTED_LOOP");
    }

    #[test]
    fn test_sync_method_detection() {
        let code = r#"
            public class Test {
                public synchronized void unsafeMethod() {
                    // heavy operation
                }
                
                public void safeMethod() {
                    synchronized(this) {
                        // block sync
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "SYNC_METHOD");
        assert!(issues[0].context.as_ref().unwrap().contains("synchronized"));
    }

    #[test]
    fn test_threadlocal_leak_detection() {
        // Case 1: Leak (set without remove)
        let leak_code = r#"
            public class LeakTest {
                private static final ThreadLocal<User> currentUser = new ThreadLocal<>();

                public void handleRequest() {
                    currentUser.set(new User());
                    // process...
                    // Missing remove()!
                }
            }
        "#;
        
        // Case 2: Safe (set with remove)
        let safe_code = r#"
            public class SafeTest {
                private static final ThreadLocal<User> context = new ThreadLocal<>();

                public void handleSafely() {
                    try {
                        context.set(new User());
                        // process...
                    } finally {
                        context.remove();
                    }
                }
            }
        "#;
        
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();

        let leak_issues = analyzer.analyze(leak_code, &PathBuf::from("LeakTest.java")).unwrap();
        assert_eq!(leak_issues.len(), 1, "Should detect leak");
        assert_eq!(leak_issues[0].id, "THREADLOCAL_LEAK");
        assert!(leak_issues[0].context.as_ref().unwrap().contains("currentUser"));

        let safe_issues = analyzer.analyze(safe_code, &PathBuf::from("SafeTest.java")).unwrap();
        assert_eq!(safe_issues.len(), 0, "Should NOT detect safe usage due to remove()");
    }
}
