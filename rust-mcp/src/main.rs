//! Java Performance Diagnostics MCP Server
//! 
//! Radar-Sniper Architecture:
//! - Phase 1: 🛰️ Radar (AST scan, 0 token)
//! - Phase 2: 🎯 Sniper (LSP verify)
//! - Phase 3: 🔬 Forensic (JDK CLI)

mod mcp;
mod ast_engine;
mod forensic;
mod jdk_engine;

use std::io::{self, BufRead, Write};
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;

fn main() {
    // 初始化日志到 stderr（MCP 协议要求 stdout 只能是 JSON-RPC）
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(io::stderr)
        .with_ansi(false)
        .init();
    
    info!("Java Perf MCP Server v4.0.0 (Rust Radar-Sniper) starting...");
    
    // MCP stdio 循环
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    for line in stdin.lock().lines() {
        match line {
            Ok(request) => {
                if request.trim().is_empty() {
                    continue;
                }
                
                match mcp::handle_request(&request) {
                    Ok(response) => {
                        writeln!(stdout, "{}", response).unwrap();
                        stdout.flush().unwrap();
                    }
                    Err(e) => {
                        error!("Error handling request: {}", e);
                        let error_response = mcp::create_error_response(&request, &e.to_string());
                        writeln!(stdout, "{}", error_response).unwrap();
                        stdout.flush().unwrap();
                    }
                }
            }
            Err(e) => {
                error!("Error reading stdin: {}", e);
                break;
            }
        }
    }
}
