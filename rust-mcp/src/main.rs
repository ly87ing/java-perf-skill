mod mcp;
mod ast_engine;
mod forensic;
mod jdk_engine;
mod checklist;
mod scanner; // v5.0 新增模块

use clap::Parser;
use mcp::McpServer;
use std::io::BufReader;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use anyhow::Result;

/// Java Performance MCP Server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(std::io::stderr) // 日志输出到 stderr，避免干扰 stdout 的 JSON-RPC
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // 启动 MCP Server
    let server = McpServer::new();
    let reader = BufReader::new(std::io::stdin());
    
    server.run(reader).await?;

    Ok(())
}
