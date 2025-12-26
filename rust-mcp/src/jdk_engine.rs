//! JDK Engine - JDK CLI 工具集成
//! 
//! 🔬 法医取证：jstack, javap, jmap

use serde_json::{json, Value};
use std::process::Command;
use std::env;

/// 检查 JDK 是否可用
pub fn check_jdk_available() -> bool {
    get_java_home().is_some()
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
fn get_jdk_tool(tool: &str) -> Option<String> {
    get_java_home().map(|home| format!("{}/bin/{}", home, tool))
}

/// 分析线程 Dump
pub fn analyze_thread_dump(pid: u32) -> Result<Value, Box<dyn std::error::Error>> {
    // 输入验证
    if pid == 0 {
        return Err("Invalid PID: 0 is not a valid process ID".into());
    }
    
    let jstack = get_jdk_tool("jstack").ok_or("JAVA_HOME not set, jstack unavailable")?;
    
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
    
    // 截取关键部分
    let lines: Vec<&str> = dump.lines().take(100).collect();
    report.push_str("### 线程摘要 (前 100 行)\n\n```\n");
    report.push_str(&lines.join("\n"));
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
    
    let javap = get_jdk_tool("javap").ok_or("JAVA_HOME not set, javap unavailable")?;
    
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
    
    let jmap = get_jdk_tool("jmap").ok_or("JAVA_HOME not set, jmap unavailable")?;
    
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
