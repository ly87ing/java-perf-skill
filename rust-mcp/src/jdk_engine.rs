//! JDK Engine - JDK CLI 工具集成
//! 
//! 🔬 法医取证：jstack, javap, jmap

use serde_json::{json, Value};
use std::process::Command;
use std::env;

/// 检查 JDK 是否可用 (旧版兼容)
pub fn check_jdk_available() -> bool {
    // 只要能找到任一工具即认为可用
    get_jdk_tool("jstack").is_some() || get_jdk_tool("jmap").is_some()
}

/// 检查单个工具的可用性
pub fn check_tool_available(tool: &str) -> bool {
    get_jdk_tool(tool).is_some()
}

/// 获取 JAVA_HOME
fn get_java_home() -> Option<String> {
    env::var("JAVA_HOME").ok().or_else(|| {
        // macOS: 尝试 /usr/libexec/java_home
        Command::new("/usr/libexec/java_home")
            .output()
            .ok()
            .and_then(|out| {
                if out.status.success() {
                    String::from_utf8(out.stdout).ok().map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
    })
}

/// 获取 JDK 工具路径
/// 优先使用 JAVA_HOME，备选使用 $PATH 中的工具
fn get_jdk_tool(tool: &str) -> Option<String> {
    // 方案 1: 使用 JAVA_HOME
    if let Some(home) = get_java_home() {
        let path = format!("{}/bin/{}", home, tool);
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }
    
    // 方案 2: 使用 which 命令在 $PATH 中查找
    Command::new("which")
        .arg(tool)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// 分析线程 Dump
pub fn analyze_thread_dump(pid: u32) -> Result<Value, Box<dyn std::error::Error>> {
    // 输入验证
    if pid == 0 {
        return Err("Invalid PID: 0 is not a valid process ID".into());
    }
    
    let jstack = get_jdk_tool("jstack").ok_or("jstack 不可用: 请确保已安装 JDK 且 JAVA_HOME 已设置或 jstack 在 $PATH 中")?;
    
    let output = Command::new(&jstack)
        .arg(pid.to_string())
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("jstack failed: {}", stderr).into());
    }
    
    let dump = String::from_utf8_lossy(&output.stdout).to_string();
    
    // 分析线程状态
    let mut blocked = 0;
    let mut waiting = 0;
    let mut runnable = 0;
    let mut deadlock = false;
    
    for line in dump.lines() {
        if line.contains("BLOCKED") {
            blocked += 1;
        } else if line.contains("WAITING") || line.contains("TIMED_WAITING") {
            waiting += 1;
        } else if line.contains("RUNNABLE") {
            runnable += 1;
        }
        
        if line.contains("Found") && line.contains("deadlock") {
            deadlock = true;
        }
    }
    
    let mut report = format!(
        "## 🔬 线程 Dump 分析 (PID: {})\n\n\
        **线程状态**:\n\
        - RUNNABLE: {}\n\
        - WAITING: {}\n\
        - BLOCKED: {}\n\n",
        pid, runnable, waiting, blocked
    );
    
    if deadlock {
        report.push_str("> [!CAUTION]\n> ⚠️ 检测到死锁！\n\n");
    }
    
    if blocked > 10 {
        report.push_str(&format!(
            "> [!WARNING]\n> {} 个线程处于 BLOCKED 状态，可能存在锁竞争\n\n",
            blocked
        ));
    }
    
    // 截取关键部分: 头部 50 行 + 尾部 50 行
    let all_lines: Vec<&str> = dump.lines().collect();
    let total_lines = all_lines.len();
    
    if total_lines <= 100 {
        // 总行数小于 100，全部显示
        report.push_str(&format!("### 线程摘要 (全部 {} 行)\n\n```\n", total_lines));
        report.push_str(&all_lines.join("\n"));
    } else {
        // 显示头尾各 50 行
        let head: Vec<&str> = all_lines.iter().take(50).cloned().collect();
        let tail: Vec<&str> = all_lines.iter().rev().take(50).cloned().collect::<Vec<_>>().into_iter().rev().collect();
        
        report.push_str(&format!("### 线程摘要 (头 50 + 尾 50 行, 共 {} 行)\n\n```\n", total_lines));
        report.push_str(&head.join("\n"));
        report.push_str(&format!("\n\n... 省略 {} 行 ...\n\n", total_lines - 100));
        report.push_str(&tail.join("\n"));
    }
    report.push_str("\n```\n");
    
    Ok(json!(report))
}

/// 分析字节码
pub fn analyze_bytecode(class_path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    // 输入验证
    if class_path.is_empty() {
        return Err("Invalid class path: path cannot be empty".into());
    }
    if class_path.contains("..") || class_path.starts_with('/') && class_path.contains(";") {
        return Err("Invalid class path: suspicious path detected".into());
    }
    
    let javap = get_jdk_tool("javap").ok_or("javap 不可用: 请确保已安装 JDK 且 JAVA_HOME 已设置或 javap 在 $PATH 中")?;
    
    let output = Command::new(&javap)
        .args(["-c", "-v", class_path])
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("javap failed: {}", stderr).into());
    }
    
    let bytecode = String::from_utf8_lossy(&output.stdout);
    
    // 截取前 200 行
    let lines: Vec<&str> = bytecode.lines().take(200).collect();
    
    let report = format!(
        "## 🔬 字节码分析: {}\n\n```\n{}\n```\n",
        class_path,
        lines.join("\n")
    );
    
    Ok(json!(report))
}

/// 分析堆内存
pub fn analyze_heap(pid: u32) -> Result<Value, Box<dyn std::error::Error>> {
    // 输入验证
    if pid == 0 {
        return Err("Invalid PID: 0 is not a valid process ID".into());
    }
    
    let jmap = get_jdk_tool("jmap").ok_or("jmap 不可用: 请确保已安装 JDK 且 JAVA_HOME 已设置或 jmap 在 $PATH 中")?;
    
    let output = Command::new(&jmap)
        .args(["-histo:live", &pid.to_string()])
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("jmap failed: {}", stderr).into());
    }
    
    let histo = String::from_utf8_lossy(&output.stdout);
    
    // 截取前 50 行（Top 对象）
    let lines: Vec<&str> = histo.lines().take(50).collect();
    
    let report = format!(
        "## 🔬 堆内存分析 (PID: {})\n\n\
        **Top 对象**:\n\n```\n{}\n```\n",
        pid,
        lines.join("\n")
    );
    
    Ok(json!(report))
}
