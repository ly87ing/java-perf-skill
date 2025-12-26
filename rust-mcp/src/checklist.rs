//! 检查清单知识库
//!
//! 来自 checklist-data.ts 的核心诊断知识

use serde::Serialize;
use serde_json::{json, Value};

/// 检查项
#[derive(Debug, Clone, Serialize)]
pub struct CheckItem {
    pub desc: String,
    pub verify: Option<String>,
    pub threshold: Option<String>,
    pub fix: Option<String>,
    pub why: Option<String>,
}

/// 检查章节
#[derive(Debug, Clone, Serialize)]
pub struct CheckSection {
    pub id: String,
    pub title: String,
    pub priority: String,
    pub items: Vec<CheckItem>,
}

/// 症状到章节的映射
pub fn get_sections_for_symptom(symptom: &str) -> Vec<&'static str> {
    match symptom {
        "memory" => vec!["5", "0", "4"],  // 内存 -> 内存与缓存, 代码放大, 资源池
        "cpu" => vec!["0", "1"],           // CPU -> 代码放大, 锁与并发
        "slow" => vec!["0", "2", "3"],     // 慢 -> 代码放大, IO阻塞, 外部调用
        "resource" => vec!["4", "2"],      // 资源 -> 资源池, IO阻塞
        "backlog" => vec!["0", "4"],       // 积压 -> 代码放大, 资源池
        "gc" => vec!["5", "0"],            // GC -> 内存缓存, 代码放大
        _ => vec![],
    }
}

/// 获取所有检查清单数据
pub fn get_checklist_data() -> Vec<CheckSection> {
    vec![
        CheckSection {
            id: "0".to_string(),
            title: "代码级放大效应".to_string(),
            priority: "P0".to_string(),
            items: vec![
                CheckItem {
                    desc: "循环内 IO/计算（for/while 内的 DB 查询、RPC）".to_string(),
                    verify: Some("grep -n \"for.*{\" | 检查内部是否有 dao/rpc 调用".to_string()),
                    threshold: None,
                    fix: Some("批量查询替代循环查询".to_string()),
                    why: Some("循环100次 x 每次10ms = 1秒".to_string()),
                },
                CheckItem {
                    desc: "集合笛卡尔积（嵌套循环 O(N*M)）".to_string(),
                    verify: Some("搜索嵌套 for 循环".to_string()),
                    threshold: Some("N*M > 10000 需优化".to_string()),
                    fix: Some("用 Map 降到 O(N+M)".to_string()),
                    why: Some("100x100=1万次".to_string()),
                },
                CheckItem {
                    desc: "频繁对象创建（循环内 new 对象）".to_string(),
                    verify: Some("async-profiler -e alloc".to_string()),
                    threshold: None,
                    fix: Some("对象池/复用".to_string()),
                    why: Some("频繁 new 导致 GC 压力".to_string()),
                },
            ],
        },
        CheckSection {
            id: "1".to_string(),
            title: "锁与并发".to_string(),
            priority: "P0".to_string(),
            items: vec![
                CheckItem {
                    desc: "锁粒度过大（synchronized 方法或大代码块）".to_string(),
                    verify: Some("jstack | grep -A 20 \"BLOCKED\"".to_string()),
                    threshold: None,
                    fix: Some("细化锁粒度/读写锁".to_string()),
                    why: Some("大锁让并发变串行".to_string()),
                },
                CheckItem {
                    desc: "死锁风险（嵌套锁获取顺序不一致）".to_string(),
                    verify: Some("jstack | grep \"deadlock\"".to_string()),
                    threshold: None,
                    fix: None,
                    why: Some("线程A持有锁1等锁2，线程B持有锁2等锁1".to_string()),
                },
            ],
        },
        CheckSection {
            id: "2".to_string(),
            title: "IO 与阻塞".to_string(),
            priority: "P0".to_string(),
            items: vec![
                CheckItem {
                    desc: "同步 IO（NIO/Netty 线程中混入阻塞操作）".to_string(),
                    verify: Some("检查 EventLoop 线程内是否有 JDBC/File IO".to_string()),
                    threshold: None,
                    fix: None,
                    why: Some("EventLoop 线程被阻塞后，该线程上的所有连接都无法处理".to_string()),
                },
                CheckItem {
                    desc: "资源未关闭（InputStream/Connection 未 close）".to_string(),
                    verify: Some("lsof -p PID | wc -l".to_string()),
                    threshold: Some("句柄 > 10000 告警".to_string()),
                    fix: Some("try-with-resources".to_string()),
                    why: Some("资源泄露导致句柄耗尽".to_string()),
                },
            ],
        },
        CheckSection {
            id: "3".to_string(),
            title: "外部调用".to_string(),
            priority: "P1".to_string(),
            items: vec![
                CheckItem {
                    desc: "无超时设置（HTTPClient, Dubbo, DB 连接）".to_string(),
                    verify: Some("搜索 timeout/connectTimeout 配置".to_string()),
                    threshold: None,
                    fix: Some("统一配置超时 3-5s".to_string()),
                    why: Some("无超时的请求可能永久等待".to_string()),
                },
                CheckItem {
                    desc: "同步串行调用（多下游串行）".to_string(),
                    verify: Some("arthas: trace 检查调用链".to_string()),
                    threshold: None,
                    fix: Some("CompletableFuture 并行".to_string()),
                    why: Some("串行 A+B+C = 300ms，并行 = max(A,B,C) = 100ms".to_string()),
                },
            ],
        },
        CheckSection {
            id: "4".to_string(),
            title: "资源池管理".to_string(),
            priority: "P0".to_string(),
            items: vec![
                CheckItem {
                    desc: "无界线程池（Executors.newCachedThreadPool）".to_string(),
                    verify: Some("arthas: thread -n 10".to_string()),
                    threshold: Some("线程 > 200 告警".to_string()),
                    fix: Some("ThreadPoolExecutor 有界".to_string()),
                    why: Some("无界池遇到流量洪峰无限创建线程".to_string()),
                },
                CheckItem {
                    desc: "池资源泄露（获取后未归还）".to_string(),
                    verify: Some("jstack | grep pool".to_string()),
                    threshold: None,
                    fix: Some("finally 归还".to_string()),
                    why: Some("每次请求泄露1个连接，池很快被占满".to_string()),
                },
            ],
        },
        CheckSection {
            id: "5".to_string(),
            title: "内存与缓存".to_string(),
            priority: "P0".to_string(),
            items: vec![
                CheckItem {
                    desc: "无界缓存（static Map 无 TTL/Size 限制）".to_string(),
                    verify: Some("jmap -histo:live | head -20".to_string()),
                    threshold: None,
                    fix: Some("Caffeine/Guava Cache".to_string()),
                    why: Some("只增不删的缓存是内存泄露".to_string()),
                },
                CheckItem {
                    desc: "ThreadLocal 泄露（请求结束未 remove）".to_string(),
                    verify: Some("搜索 ThreadLocal 未配对 remove()".to_string()),
                    threshold: None,
                    fix: Some("finally 中 remove()".to_string()),
                    why: Some("线程池复用线程，ThreadLocal 不清理导致内存累积".to_string()),
                },
                CheckItem {
                    desc: "大对象分配（一次性加载大文件/全量表）".to_string(),
                    verify: Some("MAT 分析 Dominator Tree".to_string()),
                    threshold: Some("单对象 > 10MB 关注".to_string()),
                    fix: None,
                    why: Some("大对象直接进入老年代，触发 Full GC".to_string()),
                },
            ],
        },
        CheckSection {
            id: "6".to_string(),
            title: "异常处理".to_string(),
            priority: "P2".to_string(),
            items: vec![
                CheckItem {
                    desc: "异常吞没（catch 后仅打印）".to_string(),
                    verify: Some("搜索 catch.*{.*e.printStackTrace".to_string()),
                    threshold: None,
                    fix: None,
                    why: Some("异常被吞掉导致问题难以追溯".to_string()),
                },
            ],
        },
    ]
}

/// 获取检查清单（按症状）
/// 
/// compact: true 时只返回检查项描述，省略 verify/fix/why
pub fn get_checklist(symptoms: &[&str], priority_filter: Option<&str>, compact: bool) -> Result<Value, Box<dyn std::error::Error>> {
    let all_data = get_checklist_data();
    
    // 收集相关章节ID
    let mut section_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for symptom in symptoms {
        for id in get_sections_for_symptom(symptom) {
            section_ids.insert(id);
        }
    }
    
    let mut result_sections: Vec<&CheckSection> = Vec::new();
    
    for section in &all_data {
        if section_ids.contains(section.id.as_str()) {
            // 优先级过滤
            if let Some(filter) = priority_filter {
                if filter != "all" && section.priority != filter {
                    continue;
                }
            }
            result_sections.push(section);
        }
    }
    
    // 根据 compact 模式生成不同报告
    if compact {
        // 紧凑模式
        let mut report = format!(
            "## 🔍 检查清单 (紧凑模式) - 症状: {}\n\n",
            symptoms.join(", ")
        );
        
        for section in &result_sections {
            let emoji = match section.priority.as_str() {
                "P0" => "🔴",
                "P1" => "🟡",
                _ => "🔵",
            };
            report.push_str(&format!("**{} {}**\n", emoji, section.title));
            
            for item in &section.items {
                report.push_str(&format!("- {}\n", item.desc));
            }
            report.push('\n');
        }
        
        Ok(json!(report))
    } else {
        // 完整模式
        let mut report = format!(
            "## 🔍 检查清单 (症状: {})\n\n",
            symptoms.join(", ")
        );
        
        for section in &result_sections {
            report.push_str(&format!(
                "### {} {} ({})\n\n",
                match section.priority.as_str() {
                    "P0" => "🔴",
                    "P1" => "🟡",
                    _ => "🔵",
                },
                section.title,
                section.priority
            ));
            
            for item in &section.items {
                report.push_str(&format!("- **{}**\n", item.desc));
                if let Some(verify) = &item.verify {
                    report.push_str(&format!("  - 验证: `{}`\n", verify));
                }
                if let Some(fix) = &item.fix {
                    report.push_str(&format!("  - 修复: {}\n", fix));
                }
            }
            report.push('\n');
        }
        
        Ok(json!(report))
    }
}

/// 获取所有反模式
pub fn get_all_antipatterns() -> Result<Value, Box<dyn std::error::Error>> {
    let patterns = vec![
        ("N+1 Query", "循环内执行数据库查询", "批量查询替代"),
        ("Nested Loop", "嵌套循环导致 O(N*M) 复杂度", "使用 Map/Set 优化"),
        ("ThreadLocal Leak", "ThreadLocal 未调用 remove()", "finally 中 remove()"),
        ("Unbounded Pool", "使用 newCachedThreadPool 无界池", "ThreadPoolExecutor 有界"),
        ("Unbounded Cache", "static Map 无 TTL/Size 限制", "使用 Caffeine/Guava"),
        ("Sync Method", "synchronized 方法级锁", "细化到代码块级别"),
        ("No Timeout", "HTTP/RPC 调用无超时", "统一配置 3-5s 超时"),
        ("Exception Swallow", "catch 后空处理或仅打印", "正确处理或抛出"),
        ("Resource Leak", "InputStream/Connection 未关闭", "try-with-resources"),
        ("Large Object", "一次性加载大对象 >10MB", "分页/流式处理"),
        ("Blocking IO", "NIO 线程中混入阻塞操作", "异步化处理"),
        ("CAS Spin", "高竞争 Atomic 自旋", "使用 LongAdder"),
    ];
    
    let mut report = "## ⚠️ 反模式清单\n\n".to_string();
    report.push_str("| 反模式 | 描述 | 修复建议 |\n");
    report.push_str("|--------|------|----------|\n");
    
    for (name, desc, fix) in patterns {
        report.push_str(&format!("| `{}` | {} | {} |\n", name, desc, fix));
    }
    
    Ok(json!(report))
}
