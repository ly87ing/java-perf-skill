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
            // 规则1: N_PLUS_ONE - for 循环内的调用
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
            "#, "for 循环内调用方法 (可能是 N+1 问题)"),
            
            // 规则1b: N_PLUS_ONE_WHILE - while 循环内的调用
            ("N_PLUS_ONE_WHILE", Severity::P0, r#"
                (while_statement
                    body: (block
                        (expression_statement
                            (method_invocation
                                name: (identifier) @method_name
                            ) @call
                        )
                    )
                )
            "#, "while 循环内调用方法 (可能是 N+1 问题)"),
            
            // 规则1c: N_PLUS_ONE_FOREACH - 增强型 for 循环内的调用
            ("N_PLUS_ONE_FOREACH", Severity::P0, r#"
                (enhanced_for_statement
                    body: (block
                        (expression_statement
                            (method_invocation
                                name: (identifier) @method_name
                            ) @call
                        )
                    )
                )
            "#, "foreach 循环内调用方法 (可能是 N+1 问题)"),
            
            // 规则2: NESTED_LOOP - for 嵌套 for
            ("NESTED_LOOP", Severity::P0, r#"
                (for_statement
                    body: (block
                        (for_statement) @inner_loop
                    )
                )
            "#, "嵌套 for 循环 (可能导致 O(N^2) 复杂度)"),
            
            // 规则2b: NESTED_LOOP_FOREACH - for 嵌套 enhanced_for 或反之
            ("NESTED_LOOP_MIXED", Severity::P0, r#"
                [
                    (for_statement body: (block (enhanced_for_statement) @inner_loop))
                    (enhanced_for_statement body: (block (for_statement) @inner_loop))
                    (enhanced_for_statement body: (block (enhanced_for_statement) @inner_loop))
                ]
            "#, "嵌套循环 (可能导致 O(N^2) 复杂度)"),
            
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
            
            // 规则5: STREAM_RESOURCE_LEAK - try 块内创建流但未在 finally 中关闭
            ("STREAM_RESOURCE_LEAK", Severity::P1, r#"
                (try_statement
                    body: (block
                        (local_variable_declaration
                            type: (_) @type_name
                            declarator: (variable_declarator
                                name: (identifier) @var_name
                                value: (object_creation_expression) @creation
                            )
                        )
                    )
                ) @try_block
            "#, "try 块内创建资源，请确保在 finally 中关闭或使用 try-with-resources"),
            
            // 规则6: SLEEP_IN_LOCK - synchronized 块内调用 sleep (P0)
            ("SLEEP_IN_LOCK", Severity::P0, r#"
                (synchronized_statement
                    body: (block
                        (expression_statement
                            (method_invocation
                                object: (identifier) @class_name
                                name: (identifier) @method_name
                                (#eq? @class_name "Thread")
                                (#eq? @method_name "sleep")
                            )
                        )
                    )
                ) @sync_block
            "#, "synchronized 块内调用 Thread.sleep()，持锁睡眠导致其他线程阻塞"),
            
            // 规则7: LOCK_METHOD_CALL - 检测 ReentrantLock.lock() 调用 (P0)
            ("LOCK_METHOD_CALL", Severity::P0, r#"
                (method_invocation
                    object: (identifier) @lock_var
                    name: (identifier) @method
                    (#eq? @method "lock")
                ) @lock_call
            "#, "ReentrantLock.lock() 调用，请确保 unlock() 在 finally 块中"),
            
            // ====== v7.0 AST 迁移规则 ======
            
            // 规则8: @Async 无参数 (使用默认线程池)
            ("ASYNC_DEFAULT_POOL_AST", Severity::P1, r#"
                (method_declaration
                    (modifiers
                        (marker_annotation
                            name: (identifier) @ann_name
                            (#eq? @ann_name "Async")
                        )
                    )
                ) @method
            "#, "@Async 未指定线程池，使用默认 SimpleAsyncTaskExecutor"),
            
            // 规则9: @Scheduled(fixedRate) 任务堆积风险
            ("SCHEDULED_FIXED_RATE_AST", Severity::P1, r#"
                (method_declaration
                    (modifiers
                        (annotation
                            name: (identifier) @ann_name
                            arguments: (annotation_argument_list
                                (element_value_pair
                                    key: (identifier) @key
                                    (#eq? @key "fixedRate")
                                )
                            )
                            (#eq? @ann_name "Scheduled")
                        )
                    )
                ) @method
            "#, "@Scheduled(fixedRate) 任务可能堆积，考虑使用 fixedDelay"),
            
            // 规则10: @Autowired 字段注入
            ("AUTOWIRED_FIELD_AST", Severity::P1, r#"
                (field_declaration
                    (modifiers
                        (marker_annotation
                            name: (identifier) @ann_name
                            (#eq? @ann_name "Autowired")
                        )
                    )
                ) @field
            "#, "@Autowired 字段注入不利于测试，建议使用构造器注入"),
            
            // 规则11: Flux/Mono.block() 阻塞调用
            ("FLUX_BLOCK_AST", Severity::P0, r#"
                (method_invocation
                    name: (identifier) @method_name
                    (#match? @method_name "^(block|blockFirst|blockLast)$")
                ) @call
            "#, "Flux/Mono.block() 阻塞调用，可能导致死锁"),
            
            // 规则12: subscribe() 只有一个参数 (未处理 error)
            ("SUBSCRIBE_NO_ERROR_AST", Severity::P1, r#"
                (method_invocation
                    name: (identifier) @method_name
                    arguments: (argument_list
                        (_) @first_arg
                    )
                    (#eq? @method_name "subscribe")
                ) @call
            "#, "subscribe() 可能未处理 error，建议添加 error consumer"),
            
            // 规则13: collectList() 可能导致 OOM
            ("FLUX_COLLECT_LIST_AST", Severity::P1, r#"
                (method_invocation
                    name: (identifier) @method_name
                    (#eq? @method_name "collectList")
                ) @call
            "#, "collectList() 可能导致 OOM，考虑使用 buffer 或 window"),
            
            // 规则14: parallel() 未指定 runOn
            ("PARALLEL_NO_RUN_ON_AST", Severity::P1, r#"
                (method_invocation
                    name: (identifier) @method_name
                    (#eq? @method_name "parallel")
                ) @call
            "#, "parallel() 建议配合 runOn(Schedulers.parallel()) 使用"),
            
            // ====== 更多 AST 迁移规则 (第二批) ======
            
            // 规则15: 重写 finalize() 方法 - 简化查询，只匹配方法名
            ("FINALIZE_OVERRIDE_AST", Severity::P0, r#"
                (method_declaration
                    type: (void_type)
                    name: (identifier) @method_name
                    (#eq? @method_name "finalize")
                ) @method
            "#, "重写 finalize() 已废弃，影响 GC 性能"),
            
            // 规则16: String.intern() 调用
            ("STRING_INTERN_AST", Severity::P1, r#"
                (method_invocation
                    name: (identifier) @method_name
                    (#eq? @method_name "intern")
                ) @call
            "#, "String.intern() 可能导致元空间溢出"),
            
            // 规则17: new SoftReference 使用
            ("SOFT_REFERENCE_AST", Severity::P1, r#"
                (object_creation_expression
                    type: (generic_type
                        (type_identifier) @type_name
                        (#eq? @type_name "SoftReference")
                    )
                ) @creation
            "#, "SoftReference 可能导致 Full GC 时大量对象被回收"),
            
            // 规则18: 循环内创建对象
            ("OBJECT_IN_LOOP_AST", Severity::P1, r#"
                [
                    (for_statement body: (block (local_variable_declaration declarator: (variable_declarator value: (object_creation_expression) @creation))))
                    (enhanced_for_statement body: (block (local_variable_declaration declarator: (variable_declarator value: (object_creation_expression) @creation))))
                    (while_statement body: (block (local_variable_declaration declarator: (variable_declarator value: (object_creation_expression) @creation))))
                ]
            "#, "循环内创建对象，可能导致 GC 压力"),
            
            // 规则19: @Cacheable 未指定 key
            ("CACHEABLE_NO_KEY_AST", Severity::P1, r#"
                (method_declaration
                    (modifiers
                        (annotation
                            name: (identifier) @ann_name
                            arguments: (annotation_argument_list) @args
                            (#eq? @ann_name "Cacheable")
                        )
                    )
                ) @method
            "#, "@Cacheable 建议明确指定 key 避免缓存冲突"),
            
            // 规则20: @Transactional(propagation = REQUIRES_NEW)
            ("TRANSACTIONAL_REQUIRES_NEW_AST", Severity::P1, r#"
                (method_declaration
                    (modifiers
                        (annotation
                            name: (identifier) @ann_name
                            arguments: (annotation_argument_list
                                (element_value_pair
                                    key: (identifier) @key
                                    value: (_) @value
                                    (#eq? @key "propagation")
                                )
                            )
                            (#eq? @ann_name "Transactional")
                        )
                    )
                ) @method
            "#, "@Transactional 事务传播设置，请确保理解嵌套事务行为"),
            
            // ====== 第三批 AST 迁移规则 ======
            
            // 规则21: Future.get() 无超时
            ("FUTURE_GET_NO_TIMEOUT_AST", Severity::P0, r#"
                (method_invocation
                    name: (identifier) @method_name
                    arguments: (argument_list) @args
                    (#eq? @method_name "get")
                ) @call
            "#, "Future.get() 无超时参数，可能永久阻塞"),
            
            // 规则22: await()/acquire() 无超时
            ("AWAIT_NO_TIMEOUT_AST", Severity::P0, r#"
                (method_invocation
                    name: (identifier) @method_name
                    arguments: (argument_list) @args
                    (#match? @method_name "^(await|acquire)$")
                ) @call
            "#, "await()/acquire() 无超时参数，可能永久阻塞"),
            
            // 规则23: CompletableFuture.join() 无超时
            ("COMPLETABLE_JOIN_AST", Severity::P1, r#"
                (method_invocation
                    name: (identifier) @method_name
                    (#eq? @method_name "join")
                ) @call
            "#, "CompletableFuture.join() 无超时，可能永久阻塞"),
            
            // 规则24: 日志字符串拼接
            ("LOG_STRING_CONCAT_AST", Severity::P1, r#"
                (method_invocation
                    object: (identifier) @obj
                    name: (identifier) @method_name
                    arguments: (argument_list
                        (binary_expression
                            operator: "+"
                        ) @concat
                    )
                    (#match? @obj "^(log|logger|LOG|LOGGER)$")
                    (#match? @method_name "^(debug|info|warn|error|trace)$")
                ) @call
            "#, "日志使用字符串拼接，建议使用占位符 log.info(\"x={}\", x)"),
            
            // 规则25: synchronized 代码块 (提醒检查范围)
            ("SYNC_BLOCK_AST", Severity::P1, r#"
                (synchronized_statement
                    (parenthesized_expression) @lock_obj
                    body: (block) @body
                ) @sync
            "#, "synchronized 代码块，请确保锁范围最小化"),
            
            // 规则26: EmitterProcessor.create() 无界
            ("EMITTER_UNBOUNDED_AST", Severity::P0, r#"
                (method_invocation
                    object: (identifier) @class_name
                    name: (identifier) @method_name
                    arguments: (argument_list) @args
                    (#eq? @class_name "EmitterProcessor")
                    (#eq? @method_name "create")
                ) @call
            "#, "EmitterProcessor.create() 无界背压，可能导致 OOM"),
            
            // ====== 第四批 AST 迁移规则 (最终批次) ======
            
            // 规则27: Executors.newCachedThreadPool 等无界线程池
            ("UNBOUNDED_POOL_AST", Severity::P0, r#"
                (method_invocation
                    object: (identifier) @class_name
                    name: (identifier) @method_name
                    (#eq? @class_name "Executors")
                    (#match? @method_name "^(newCachedThreadPool|newScheduledThreadPool|newSingleThreadExecutor)$")
                ) @call
            "#, "Executors 无界线程池，建议使用 ThreadPoolExecutor 配置有界队列"),
            
            // 规则28: 空 catch 块
            ("EMPTY_CATCH_AST", Severity::P0, r#"
                (catch_clause
                    body: (block) @body
                ) @catch
            "#, "catch 块可能为空或仅打印，请正确处理异常"),
            
            // 规则29: new FileInputStream/FileOutputStream
            ("BLOCKING_IO_AST", Severity::P1, r#"
                (object_creation_expression
                    type: (type_identifier) @type_name
                    (#match? @type_name "^File(Input|Output)Stream$")
                ) @creation
            "#, "FileInputStream/FileOutputStream 同步阻塞 IO，考虑使用 NIO"),
            
            // 规则30: AtomicInteger/AtomicLong 高竞争
            ("ATOMIC_SPIN_AST", Severity::P1, r#"
                (object_creation_expression
                    type: (type_identifier) @type_name
                    (#match? @type_name "^Atomic(Integer|Long)$")
                ) @creation
            "#, "AtomicInteger/Long 高竞争时考虑使用 LongAdder"),
            
            // 规则31: Sinks.many() 无背压
            ("SINKS_MANY_AST", Severity::P1, r#"
                (method_invocation
                    object: (identifier) @class_name
                    name: (identifier) @method_name
                    (#eq? @class_name "Sinks")
                    (#eq? @method_name "many")
                ) @call
            "#, "Sinks.many() 需要配置背压策略"),
            
            // 规则32: Caffeine/CacheBuilder.newBuilder()
            ("CACHE_NO_EXPIRE_AST", Severity::P1, r#"
                (method_invocation
                    object: (identifier) @class_name
                    name: (identifier) @method_name
                    (#match? @class_name "^(Caffeine|CacheBuilder)$")
                    (#eq? @method_name "newBuilder")
                ) @call
            "#, "Cache.newBuilder() 请确保配置了过期策略和最大大小"),
            
            // 规则33: static Map/List/Set 无界缓存
            ("STATIC_COLLECTION_AST", Severity::P0, r#"
                (field_declaration
                    (modifiers) @mods
                    type: (generic_type
                        (type_identifier) @type_name
                        (#match? @type_name "^(Map|HashMap|ConcurrentHashMap|List|ArrayList|Set|HashSet)$")
                    )
                ) @field
            "#, "static 集合作为缓存需配置大小限制和过期策略"),
            
            // 规则34: DriverManager.getConnection 直连
            ("DATASOURCE_NO_POOL_AST", Severity::P1, r#"
                (method_invocation
                    object: (identifier) @class_name
                    name: (identifier) @method_name
                    (#eq? @class_name "DriverManager")
                    (#eq? @method_name "getConnection")
                ) @call
            "#, "DriverManager.getConnection 直接获取连接，建议使用连接池"),
            
            // ====== 最终批次 AST 规则 ======
            
            // 规则35: 循环内字符串 += 拼接
            ("STRING_CONCAT_LOOP_AST", Severity::P1, r#"
                [
                    (for_statement body: (block (expression_statement (assignment_expression left: (_) @var operator: "+=" right: (_) @value)) @assign))
                    (enhanced_for_statement body: (block (expression_statement (assignment_expression left: (_) @var operator: "+=" right: (_) @value)) @assign))
                    (while_statement body: (block (expression_statement (assignment_expression left: (_) @var operator: "+=" right: (_) @value)) @assign))
                ]
            "#, "循环内使用 += 拼接字符串，建议使用 StringBuilder"),
            
            // 规则36: 大数组分配 new byte[1000000]
            ("LARGE_ARRAY_AST", Severity::P1, r#"
                (array_creation_expression
                    type: (integral_type) @type_name
                    dimensions: (dimensions_expr
                        (decimal_integer_literal) @size
                    )
                ) @creation
            "#, "大数组分配可能导致 Full GC，考虑对象池或分块处理"),
            
            // ====== v8.0 Java 现代化规则 ======
            
            // 规则37: Virtual Threads Pinning 风险 (synchronized 在 Virtual Thread 下)
            ("VIRTUAL_THREAD_PINNING", Severity::P0, r#"
                (synchronized_statement) @sync
            "#, "[Virtual Threads] synchronized 会导致 Carrier Thread Pinning，考虑使用 ReentrantLock"),
            
            // 规则38: GraalVM Class.forName 检测
            ("GRAALVM_CLASS_FORNAME", Severity::P1, r#"
                (method_invocation
                    object: (identifier) @class_name
                    name: (identifier) @method_name
                    (#eq? @class_name "Class")
                    (#eq? @method_name "forName")
                ) @call
            "#, "[GraalVM] Class.forName 需要配置 reflect-config.json"),
            
            // 规则39: GraalVM Method.invoke 检测
            ("GRAALVM_METHOD_INVOKE", Severity::P1, r#"
                (method_invocation
                    name: (identifier) @method_name
                    (#eq? @method_name "invoke")
                ) @call
            "#, "[GraalVM] Method.invoke 需要配置反射元数据"),
            
            // 规则40: GraalVM Proxy.newProxyInstance 检测
            ("GRAALVM_PROXY", Severity::P1, r#"
                (method_invocation
                    object: (identifier) @class_name
                    name: (identifier) @method_name
                    (#eq? @class_name "Proxy")
                    (#eq? @method_name "newProxyInstance")
                ) @call
            "#, "[GraalVM] Proxy.newProxyInstance 需要配置 proxy-config.json"),
        ];

        let mut compiled = Vec::with_capacity(rule_defs.len());
        
        for (id, severity, query_str, description) in rule_defs {
            let query = Query::new(language, query_str)
                .map_err(|e| anyhow!("Failed to compile query for {id}: {e}"))?;
            
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
        parser.set_language(&self.language).map_err(|e| anyhow!("Failed to set language: {e}"))?;

        let tree = parser.parse(code, None).ok_or_else(|| anyhow!("Failed to parse code"))?;
        let root_node = tree.root_node();
        let mut issues = Vec::new();

        // 使用预编译的查询 (不再每次编译)
        for rule in &self.compiled_rules {
            let mut query_cursor = QueryCursor::new();
            let matches = query_cursor.matches(&rule.query, root_node, code.as_bytes());

            for m in matches {
                match rule.id {
                    // N+1 检测：支持 for, while, foreach 三种循环
                    "N_PLUS_ONE" | "N_PLUS_ONE_WHILE" | "N_PLUS_ONE_FOREACH" => {
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

                        // 检查是否是 DAO/RPC 方法名
                        if method_name_text.contains("find") || 
                           method_name_text.contains("save") || 
                           method_name_text.contains("select") || 
                           method_name_text.contains("delete") ||
                           method_name_text.contains("get") ||
                           method_name_text.contains("query") ||
                           method_name_text.contains("load") ||
                           method_name_text.contains("fetch") {
                            
                            let file_name = file_path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "unknown".to_string());

                            // 统一 ID 为 N_PLUS_ONE，便于上层处理
                            issues.push(Issue {
                                id: "N_PLUS_ONE".to_string(),
                                severity: rule.severity,
                                file: file_name,
                                line,
                                description: format!("{} (Method: {})", rule.description, method_name_text),
                                context: Some(method_name_text),
                            });
                        }
                    },
                    // 嵌套循环检测：支持 for-for, for-foreach, foreach-for, foreach-foreach
                    "NESTED_LOOP" | "NESTED_LOOP_MIXED" => {
                        let inner_loop_idx = rule.query.capture_index_for_name("inner_loop").unwrap();
                        for capture in m.captures {
                            if capture.index == inner_loop_idx {
                                let line = capture.node.start_position().row + 1;
                                // 统一 ID 为 NESTED_LOOP
                                issues.push(Issue {
                                    id: "NESTED_LOOP".to_string(),
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
                                let remove_call = format!("{var_name}.remove()");
                                
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
                    "STREAM_RESOURCE_LEAK" => {
                        // 检测 try 块内创建的流资源
                        if let Some(type_idx) = rule.query.capture_index_for_name("type_name") {
                            if let Some(var_idx) = rule.query.capture_index_for_name("var_name") {
                                let mut type_name = String::new();
                                let mut var_name = String::new();
                                let mut line = 0;

                                for capture in m.captures {
                                    if capture.index == type_idx {
                                        type_name = capture.node.utf8_text(code.as_bytes()).unwrap_or("").to_string();
                                    }
                                    if capture.index == var_idx {
                                        var_name = capture.node.utf8_text(code.as_bytes()).unwrap_or("").to_string();
                                        line = capture.node.start_position().row + 1;
                                    }
                                }

                                // 只关注流类型
                                if type_name.contains("Stream") || 
                                   type_name.contains("Reader") || 
                                   type_name.contains("Writer") ||
                                   type_name.contains("Connection") ||
                                   type_name.contains("Socket") {
                                    issues.push(Issue {
                                        id: rule.id.to_string(),
                                        severity: rule.severity,
                                        file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                        line,
                                        description: format!("{} (Type: {}, Var: {})", rule.description, type_name, var_name),
                                        context: Some(var_name),
                                    });
                                }
                            }
                        }
                    },
                    "SLEEP_IN_LOCK" => {
                        // 检测 synchronized 块内的 Thread.sleep()
                        if let Some(sync_idx) = rule.query.capture_index_for_name("sync_block") {
                            for capture in m.captures {
                                if capture.index == sync_idx {
                                    let line = capture.node.start_position().row + 1;
                                    issues.push(Issue {
                                        id: rule.id.to_string(),
                                        severity: rule.severity,
                                        file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                        line,
                                        description: rule.description.to_string(),
                                        context: Some("Thread.sleep() in synchronized".to_string()),
                                    });
                                }
                            }
                        }
                    },
                    "LOCK_METHOD_CALL" => {
                        // 检测 ReentrantLock.lock() 调用
                        if let Some(lock_idx) = rule.query.capture_index_for_name("lock_call") {
                            if let Some(var_idx) = rule.query.capture_index_for_name("lock_var") {
                                let mut lock_var = String::new();
                                let mut line = 0;

                                for capture in m.captures {
                                    if capture.index == var_idx {
                                        lock_var = capture.node.utf8_text(code.as_bytes()).unwrap_or("").to_string();
                                    }
                                    if capture.index == lock_idx {
                                        line = capture.node.start_position().row + 1;
                                    }
                                }

                                // 检查方法内是否有配对的 unlock()
                                // 向上查找 method_declaration
                                if let Some(lock_node) = m.captures.iter().find(|c| c.index == lock_idx).map(|c| c.node) {
                                    let mut current = lock_node.parent();
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
                                        let unlock_in_finally = format!("{lock_var}.unlock()");
                                        let has_finally = method_text.contains("finally");
                                        
                                        // 如果没有 finally 块或 finally 中没有 unlock
                                        if !has_finally || !method_text.contains(&unlock_in_finally) {
                                            issues.push(Issue {
                                                id: rule.id.to_string(),
                                                severity: rule.severity,
                                                file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                                line,
                                                description: format!("{} (Lock: {})", rule.description, lock_var),
                                                context: Some(lock_var),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    },
                    // v7.0 AST 迁移规则 - 通用处理
                    "ASYNC_DEFAULT_POOL_AST" | "SCHEDULED_FIXED_RATE_AST" | "AUTOWIRED_FIELD_AST" => {
                        // Spring 注解规则 - 匹配 @method 或 @field
                        let target_idx = rule.query.capture_index_for_name("method")
                            .or_else(|| rule.query.capture_index_for_name("field"));
                        
                        if let Some(idx) = target_idx {
                            for capture in m.captures {
                                if capture.index == idx {
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
                        }
                    },
                    "FLUX_BLOCK_AST" | "SUBSCRIBE_NO_ERROR_AST" | "FLUX_COLLECT_LIST_AST" | "PARALLEL_NO_RUN_ON_AST" => {
                        // 响应式编程规则 - 匹配 @call
                        if let Some(call_idx) = rule.query.capture_index_for_name("call") {
                            for capture in m.captures {
                                if capture.index == call_idx {
                                    let line = capture.node.start_position().row + 1;
                                    let method_text = capture.node.utf8_text(code.as_bytes()).unwrap_or("").to_string();
                                    issues.push(Issue {
                                        id: rule.id.to_string(),
                                        severity: rule.severity,
                                        file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                        line,
                                        description: rule.description.to_string(),
                                        context: Some(method_text),
                                    });
                                }
                            }
                        }
                    },
                    // 第二批 AST 规则 - GC 和 Spring 相关
                    "FINALIZE_OVERRIDE_AST" | "CACHEABLE_NO_KEY_AST" | "TRANSACTIONAL_REQUIRES_NEW_AST" => {
                        // 方法级规则 - 匹配 @method
                        if let Some(method_idx) = rule.query.capture_index_for_name("method") {
                            for capture in m.captures {
                                if capture.index == method_idx {
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
                        }
                    },
                    "STRING_INTERN_AST" => {
                        // intern() 调用 - 匹配 @call
                        if let Some(call_idx) = rule.query.capture_index_for_name("call") {
                            for capture in m.captures {
                                if capture.index == call_idx {
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
                        }
                    },
                    "SOFT_REFERENCE_AST" | "OBJECT_IN_LOOP_AST" => {
                        // 对象创建规则 - 匹配 @creation
                        if let Some(creation_idx) = rule.query.capture_index_for_name("creation") {
                            for capture in m.captures {
                                if capture.index == creation_idx {
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
                        }
                    },
                    // 第三批 AST 规则 - 阻塞调用和锁
                    "FUTURE_GET_NO_TIMEOUT_AST" | "AWAIT_NO_TIMEOUT_AST" | "COMPLETABLE_JOIN_AST" | "EMITTER_UNBOUNDED_AST" => {
                        // 方法调用检测 - 检查参数列表是否为空
                        if let Some(call_idx) = rule.query.capture_index_for_name("call") {
                            if let Some(args_idx) = rule.query.capture_index_for_name("args") {
                                let mut args_node = None;
                                let mut line = 0;
                                
                                for capture in m.captures {
                                    if capture.index == args_idx {
                                        args_node = Some(capture.node);
                                    }
                                    if capture.index == call_idx {
                                        line = capture.node.start_position().row + 1;
                                    }
                                }
                                
                                // 只有参数列表为空时才报告 (无超时)
                                if let Some(args) = args_node {
                                    if args.child_count() <= 2 { // 只有 ( 和 )
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
                            }
                        }
                    },
                    "LOG_STRING_CONCAT_AST" => {
                        // 日志字符串拼接检测
                        if let Some(call_idx) = rule.query.capture_index_for_name("call") {
                            for capture in m.captures {
                                if capture.index == call_idx {
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
                        }
                    },
                    "SYNC_BLOCK_AST" => {
                        // synchronized 代码块检测
                        if let Some(sync_idx) = rule.query.capture_index_for_name("sync") {
                            for capture in m.captures {
                                if capture.index == sync_idx {
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
                        }
                    },
                    // 第四批 AST 规则 - Executors/Catch/IO/Atomic/Sinks/Cache
                    "UNBOUNDED_POOL_AST" | "SINKS_MANY_AST" | "CACHE_NO_EXPIRE_AST" | "DATASOURCE_NO_POOL_AST" => {
                        // 方法调用类规则 - 匹配 @call
                        if let Some(call_idx) = rule.query.capture_index_for_name("call") {
                            for capture in m.captures {
                                if capture.index == call_idx {
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
                        }
                    },
                    "EMPTY_CATCH_AST" => {
                        // 空 catch 块检测 - 检查 body 是否为空
                        if let Some(catch_idx) = rule.query.capture_index_for_name("catch") {
                            if let Some(body_idx) = rule.query.capture_index_for_name("body") {
                                let mut body_node = None;
                                let mut line = 0;
                                
                                for capture in m.captures {
                                    if capture.index == body_idx {
                                        body_node = Some(capture.node);
                                    }
                                    if capture.index == catch_idx {
                                        line = capture.node.start_position().row + 1;
                                    }
                                }
                                
                                // 只有 { } 或只有空白/注释时报告
                                if let Some(body) = body_node {
                                    let body_text = body.utf8_text(code.as_bytes()).unwrap_or("{}");
                                    let inner = body_text.trim_start_matches('{').trim_end_matches('}').trim();
                                    // 空或只有打印语句
                                    if inner.is_empty() || inner.contains(".print") {
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
                            }
                        }
                    },
                    "BLOCKING_IO_AST" | "ATOMIC_SPIN_AST" => {
                        // 对象创建类规则
                        if let Some(creation_idx) = rule.query.capture_index_for_name("creation") {
                            for capture in m.captures {
                                if capture.index == creation_idx {
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
                        }
                    },
                    "STATIC_COLLECTION_AST" => {
                        // static 集合检测 - 检查 modifiers 是否包含 static
                        if let Some(field_idx) = rule.query.capture_index_for_name("field") {
                            if let Some(mods_idx) = rule.query.capture_index_for_name("mods") {
                                let mut is_static = false;
                                let mut line = 0;
                                
                                for capture in m.captures {
                                    if capture.index == mods_idx {
                                        let mods_text = capture.node.utf8_text(code.as_bytes()).unwrap_or("");
                                        is_static = mods_text.contains("static");
                                    }
                                    if capture.index == field_idx {
                                        line = capture.node.start_position().row + 1;
                                    }
                                }
                                
                                if is_static {
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
                        }
                    },
                    // 最终批次 AST 规则
                    "STRING_CONCAT_LOOP_AST" => {
                        // 循环内 += 拼接检测
                        if let Some(assign_idx) = rule.query.capture_index_for_name("assign") {
                            for capture in m.captures {
                                if capture.index == assign_idx {
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
                        }
                    },
                    "LARGE_ARRAY_AST" => {
                        // 大数组分配检测 - 检查数组大小
                        if let Some(creation_idx) = rule.query.capture_index_for_name("creation") {
                            if let Some(size_idx) = rule.query.capture_index_for_name("size") {
                                let mut size_value: i64 = 0;
                                let mut line = 0;
                                
                                for capture in m.captures {
                                    if capture.index == size_idx {
                                        let size_text = capture.node.utf8_text(code.as_bytes()).unwrap_or("0");
                                        size_value = size_text.parse().unwrap_or(0);
                                    }
                                    if capture.index == creation_idx {
                                        line = capture.node.start_position().row + 1;
                                    }
                                }
                                
                                // 只有大于 1MB (1_000_000 bytes) 才报告
                                if size_value >= 1_000_000 {
                                    issues.push(Issue {
                                        id: rule.id.to_string(),
                                        severity: rule.severity,
                                        file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                        line,
                                        description: format!("{} (size: {})", rule.description, size_value),
                                        context: None,
                                    });
                                }
                            }
                        }
                    },
                    // v8.0 Java 现代化规则
                    "VIRTUAL_THREAD_PINNING" => {
                        // Virtual Threads Pinning 检测
                        if let Some(sync_idx) = rule.query.capture_index_for_name("sync") {
                            for capture in m.captures {
                                if capture.index == sync_idx {
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
                        }
                    },
                    "GRAALVM_CLASS_FORNAME" | "GRAALVM_METHOD_INVOKE" | "GRAALVM_PROXY" => {
                        // GraalVM 反射检测
                        if let Some(call_idx) = rule.query.capture_index_for_name("call") {
                            for capture in m.captures {
                                if capture.index == call_idx {
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

        // 现在会检测到: SYNC_METHOD + SYNC_BLOCK_AST + VIRTUAL_THREAD_PINNING
        assert_eq!(issues.len(), 3, "Should detect SYNC_METHOD, SYNC_BLOCK_AST, VIRTUAL_THREAD_PINNING");
        assert!(issues.iter().any(|i| i.id == "SYNC_METHOD"), "Should detect SYNC_METHOD");
        assert!(issues.iter().any(|i| i.id == "SYNC_BLOCK_AST" || i.id == "VIRTUAL_THREAD_PINNING"), "Should detect sync-related issues");
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

    #[test]
    fn test_n_plus_one_while_loop() {
        let code = r#"
            public class Test {
                public void process() {
                    Iterator<User> it = users.iterator();
                    while (it.hasNext()) {
                        User u = it.next();
                        orderDao.findByUserId(u.getId());
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert!(issues.iter().any(|i| i.id == "N_PLUS_ONE"), "Should detect N+1 in while loop");
    }

    #[test]
    fn test_n_plus_one_foreach_loop() {
        let code = r#"
            public class Test {
                public void process(List<User> users) {
                    for (User user : users) {
                        userRepository.save(user);
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert!(issues.iter().any(|i| i.id == "N_PLUS_ONE"), "Should detect N+1 in foreach loop");
    }

    #[test]
    fn test_nested_loop_foreach_mixed() {
        let code = r#"
            public class Test {
                public void process(List<User> users, List<Order> orders) {
                    for (User user : users) {
                        for (Order order : orders) {
                            // O(N*M) 复杂度
                        }
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert!(issues.iter().any(|i| i.id == "NESTED_LOOP"), "Should detect nested foreach loops");
    }

    #[test]
    fn test_sleep_in_lock() {
        let code = r#"
            public class Test {
                private final Object lock = new Object();
                
                public void badMethod() {
                    synchronized(lock) {
                        Thread.sleep(1000);
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert!(issues.iter().any(|i| i.id == "SLEEP_IN_LOCK"), "Should detect Thread.sleep() in synchronized block");
    }

    #[test]
    fn test_reentrant_lock_leak() {
        // Case 1: Leak (lock without finally unlock)
        let leak_code = r#"
            public class Test {
                private ReentrantLock myLock = new ReentrantLock();
                
                public void badMethod() {
                    myLock.lock();
                    doSomething();
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(leak_code, &file).unwrap();

        // 打印调试信息
        for issue in &issues {
            println!("Found issue: {} - {}", issue.id, issue.description);
        }

        assert!(issues.iter().any(|i| i.id == "LOCK_METHOD_CALL"), "Should detect lock() without finally unlock()");
    }

    #[test]
    fn test_reentrant_lock_safe() {
        // Case 2: Safe (lock with finally unlock)
        let safe_code = r#"
            public class Test {
                private ReentrantLock lock = new ReentrantLock();
                
                public void safeMethod() {
                    lock.lock();
                    try {
                        doSomething();
                    } finally {
                        lock.unlock();
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(safe_code, &file).unwrap();

        assert!(!issues.iter().any(|i| i.id == "LOCK_METHOD_CALL"), "Should NOT detect when unlock() is in finally");
    }

    // ====== v7.0 AST 迁移规则测试 ======

    #[test]
    fn test_async_default_pool() {
        let code = r#"
            @Service
            public class MyService {
                @Async
                public void asyncMethod() {
                    // uses default SimpleAsyncTaskExecutor
                }
                
                @Async("customExecutor")
                public void asyncWithPool() {
                    // uses custom pool - should NOT trigger
                }
            }
        "#;
        
        let file = PathBuf::from("MyService.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert!(issues.iter().any(|i| i.id == "ASYNC_DEFAULT_POOL_AST"), "Should detect @Async without pool");
    }

    #[test]
    fn test_autowired_field() {
        let code = r#"
            @Service
            public class MyService {
                @Autowired
                private UserRepository userRepo;
                
                private final OrderRepository orderRepo;
                
                public MyService(OrderRepository orderRepo) {
                    this.orderRepo = orderRepo;
                }
            }
        "#;
        
        let file = PathBuf::from("MyService.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert!(issues.iter().any(|i| i.id == "AUTOWIRED_FIELD_AST"), "Should detect @Autowired field injection");
    }

    #[test]
    fn test_flux_block() {
        let code = r#"
            public class ReactiveService {
                public User getUser() {
                    return userClient.getUser().block();
                }
                
                public User getFirstUser() {
                    return userClient.getUsers().blockFirst();
                }
            }
        "#;
        
        let file = PathBuf::from("ReactiveService.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        let block_issues: Vec<_> = issues.iter().filter(|i| i.id == "FLUX_BLOCK_AST").collect();
        assert_eq!(block_issues.len(), 2, "Should detect both block() and blockFirst()");
    }

    #[test]
    fn test_subscribe_no_error() {
        let code = r#"
            public class ReactiveService {
                public void process() {
                    flux.subscribe(data -> handle(data));
                }
            }
        "#;
        
        let file = PathBuf::from("ReactiveService.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert!(issues.iter().any(|i| i.id == "SUBSCRIBE_NO_ERROR_AST"), "Should detect subscribe() without error handler");
    }
}
