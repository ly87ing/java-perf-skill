// ============================================================================
// RuleHandler Trait - 规则处理器抽象
// ============================================================================
//
// v9.2: 解耦规则处理逻辑，遵循开闭原则
//
// 之前的问题：analyze_with_context 中有大量 match rule.id 分支
// 每次添加新规则都要修改这个大 match
//
// 解决方案：
// 1. 定义 RuleHandler trait
// 2. 每种规则类型实现自己的 Handler
// 3. CompiledRule 持有 Box<dyn RuleHandler>
//
// ============================================================================

use tree_sitter::{Query, QueryMatch};
use super::{Issue, Severity};
use crate::symbol_table::SymbolTable;
use std::path::Path;

/// 规则处理上下文
pub struct RuleContext<'a> {
    pub code: &'a str,
    pub file_path: &'a Path,
    pub current_class: &'a str,
    pub symbol_table: Option<&'a SymbolTable>,
}

/// 规则处理器 trait
pub trait RuleHandler: Send + Sync {
    /// 处理匹配结果，返回检测到的问题（如果有）
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue>;
}

// ============================================================================
// 通用处理器实现
// ============================================================================

/// 简单匹配处理器 - 只需要报告匹配位置
pub struct SimpleMatchHandler {
    /// 用于获取行号的 capture 名称
    pub line_capture: &'static str,
}

impl RuleHandler for SimpleMatchHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let capture_idx = query.capture_index_for_name(self.line_capture)?;

        for capture in m.captures {
            if capture.index == capture_idx {
                let line = capture.node.start_position().row + 1;
                return Some(Issue {
                    id: rule_id.to_string(),
                    severity,
                    file: ctx.file_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    line,
                    description: description.to_string(),
                    context: None,
                });
            }
        }
        None
    }
}

/// 字符串内容匹配处理器 - 用于 SQL 检测等
pub struct StringContentHandler {
    pub string_capture: &'static str,
    pub max_context_len: usize,
}

impl RuleHandler for StringContentHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let str_idx = query.capture_index_for_name(self.string_capture)?;

        for capture in m.captures {
            if capture.index == str_idx {
                let line = capture.node.start_position().row + 1;
                let str_content = capture.node.utf8_text(ctx.code.as_bytes()).unwrap_or("");
                let context = if str_content.len() > self.max_context_len {
                    format!("{}...", &str_content[..self.max_context_len])
                } else {
                    str_content.to_string()
                };

                return Some(Issue {
                    id: rule_id.to_string(),
                    severity,
                    file: ctx.file_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    line,
                    description: description.to_string(),
                    context: Some(context),
                });
            }
        }
        None
    }
}

/// 修饰符检查处理器 - 检查 synchronized, volatile 等
pub struct ModifierCheckHandler {
    pub mods_capture: &'static str,
    pub target_capture: &'static str,
    pub required_modifier: &'static str,
}

impl RuleHandler for ModifierCheckHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let mods_idx = query.capture_index_for_name(self.mods_capture)?;
        let target_idx = query.capture_index_for_name(self.target_capture)?;

        let mut has_modifier = false;
        let mut line = 0;

        for capture in m.captures {
            if capture.index == mods_idx {
                let mods_text = capture.node.utf8_text(ctx.code.as_bytes()).unwrap_or("");
                has_modifier = mods_text.contains(self.required_modifier);
            }
            if capture.index == target_idx {
                line = capture.node.start_position().row + 1;
            }
        }

        if has_modifier && line > 0 {
            Some(Issue {
                id: rule_id.to_string(),
                severity,
                file: ctx.file_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                line,
                description: description.to_string(),
                context: None,
            })
        } else {
            None
        }
    }
}

/// N+1 检测处理器 - 带语义分析
pub struct NPlusOneHandler;

impl RuleHandler for NPlusOneHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let method_name_idx = query.capture_index_for_name("method_name")?;
        let call_idx = query.capture_index_for_name("call")?;

        let mut method_name_text = String::new();
        let mut line = 0;
        let mut call_node = None;

        for capture in m.captures {
            if capture.index == method_name_idx {
                method_name_text = capture.node.utf8_text(ctx.code.as_bytes())
                    .unwrap_or("").to_string();
            }
            if capture.index == call_idx {
                line = capture.node.start_position().row + 1;
                call_node = Some(capture.node);
            }
        }

        // 获取 receiver
        let mut receiver_name = String::new();
        if let Some(node) = call_node {
            if let Some(obj_node) = node.child_by_field_name("object") {
                receiver_name = obj_node.utf8_text(ctx.code.as_bytes())
                    .unwrap_or("").to_string();
            }
        }

        let is_suspicious = if let Some(symbol_table) = ctx.symbol_table {
            // Semantic Mode
            if !receiver_name.is_empty() {
                symbol_table.is_dao_call(ctx.current_class, &receiver_name, &method_name_text)
            } else {
                // Fallback
                method_name_text.contains("find") || method_name_text.contains("save")
            }
        } else {
            // Heuristic Mode
            Self::is_dao_method(&method_name_text) || Self::is_dao_receiver(&receiver_name)
        };

        if is_suspicious {
            Some(Issue {
                id: rule_id.to_string(),
                severity,
                file: ctx.file_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                line,
                description: description.to_string(),
                context: Some(format!("{}.{}()", receiver_name, method_name_text)),
            })
        } else {
            None
        }
    }
}

impl NPlusOneHandler {
    fn is_dao_method(method_name: &str) -> bool {
        let dao_patterns = [
            "findBy", "findAll", "findOne", "findById",
            "saveAll", "saveAndFlush",
            "deleteBy", "deleteAll", "deleteById",
            "selectBy", "selectAll", "selectOne", "selectList",
            "queryBy", "queryFor", "queryAll",
            "loadBy", "loadAll", "fetchBy", "fetchAll",
        ];
        dao_patterns.iter().any(|p| method_name.starts_with(p))
    }

    fn is_dao_receiver(receiver: &str) -> bool {
        let dao_suffixes = ["Repository", "Dao", "Mapper", "repo", "dao", "mapper"];
        dao_suffixes.iter().any(|s| receiver.ends_with(s) || receiver.contains(s))
    }
}

// ============================================================================
// 处理器工厂
// ============================================================================

/// 根据规则 ID 创建对应的处理器
pub fn create_handler(rule_id: &str) -> Box<dyn RuleHandler> {
    match rule_id {
        // N+1 检测
        "N_PLUS_ONE" | "N_PLUS_ONE_WHILE" | "N_PLUS_ONE_FOREACH" => {
            Box::new(NPlusOneHandler)
        }

        // SQL 字符串检测
        "SELECT_STAR" | "LIKE_LEADING_WILDCARD" => {
            Box::new(StringContentHandler {
                string_capture: "str",
                max_context_len: 50,
            })
        }

        // synchronized 方法
        "SYNC_METHOD" => {
            Box::new(ModifierCheckHandler {
                mods_capture: "mods",
                target_capture: "method",
                required_modifier: "synchronized",
            })
        }

        // volatile 数组
        "VOLATILE_ARRAY" => {
            Box::new(ModifierCheckHandler {
                mods_capture: "mods",
                target_capture: "field",
                required_modifier: "volatile",
            })
        }

        // 默认使用简单匹配
        _ => {
            // 尝试常见的 capture 名称
            Box::new(SimpleMatchHandler {
                line_capture: "call",
            })
        }
    }
}
