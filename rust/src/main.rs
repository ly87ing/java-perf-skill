mod ast_engine;
mod forensic;
mod jdk_engine;
mod checklist;
mod scanner;
mod cli;
mod taint;

use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use anyhow::Result;

/// Java Performance Diagnostics Tool
///
/// CLI 工具，通过 Bash 调用，默认输出人类可读格式
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "info")]
    log_level: String,

    /// 输出 JSON 格式 (默认输出人类可读的 Markdown)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// 🛰️ 雷达扫描 - 全项目 AST 分析
    Scan {
        /// 项目路径
        #[arg(short, long, default_value = ".")]
        path: String,

        /// 显示完整结果（默认只显示 P0）
        #[arg(long)]
        full: bool,

        /// 最多返回的 P1 数量 (--full 模式)
        #[arg(long, default_value = "5")]
        max_p1: usize,
    },

    /// 🔍 单文件分析
    Analyze {
        /// 文件路径
        #[arg(short, long)]
        file: String,
    },

    /// 📋 获取检查清单
    Checklist {
        /// 症状列表 (逗号分隔): memory,cpu,slow,resource,backlog,gc
        #[arg(short, long)]
        symptoms: String,

        /// 显示完整信息（默认紧凑模式）
        #[arg(long)]
        full: bool,
    },

    /// ⚠️ 列出所有反模式
    Antipatterns,

    /// 🔬 分析日志文件
    Log {
        /// 日志文件路径
        #[arg(short, long)]
        file: String,
    },

    /// 🔬 分析线程 Dump (jstack)
    Jstack {
        /// Java 进程 PID
        #[arg(short, long)]
        pid: u32,
    },

    /// 🔬 分析字节码 (javap)
    Javap {
        /// 类路径或 .class 文件
        #[arg(short, long)]
        class: String,
    },

    /// 🔬 分析堆内存 (jmap)
    Jmap {
        /// Java 进程 PID
        #[arg(short, long)]
        pid: u32,
    },

    /// 📋 项目摘要
    Summary {
        /// 项目路径
        #[arg(short, long, default_value = ".")]
        path: String,
    },

    /// ℹ️ 引擎状态
    Status,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    cli::handle_command(args.command, args.json)
}
