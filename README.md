# Java Perf v4.1.0 (Rust)

<p align="center">
  <img src="https://img.shields.io/badge/Version-4.1.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/Language-Rust-orange" alt="Rust">
  <img src="https://img.shields.io/badge/Size-1.9MB-green" alt="Binary Size">
  <img src="https://img.shields.io/badge/Dependencies-Zero-purple" alt="No Dependencies">
  <img src="https://img.shields.io/badge/Tools-9-blue" alt="MCP Tools">
</p>

A Claude Skill + MCP Server for diagnosing Java performance issues using the **Radar-Sniper Architecture**.

**Now powered by Rust 🦀 for extreme performance!**

## 🏆 Architecture

```
Phase 1: 🛰️ Radar (Zero Cost)
└── Rust AST Engine - Millisecond-level full project scan
    ├── Static Regex Compilation (Lazy)
    ├── Comment Filtering
    └── 17+ Performance Rules (P0/P1)

Phase 2: 🎯 Sniper (Verification)
└── Verify context and provide fixes

Phase 3: 🔬 Forensic (Deep Dive)
└── JDK CLI Integration - jstack/javap/jmap support
```

## 🚀 Advantages

| Metric | Node.js (v3.x) | Rust (v4.0+) |
|--------|---------------|-------------|
| **Install** | Need Node.js | **Zero Dependencies** (Release Download) |
| **Size** | ~50MB | **~1.9MB** (Single Binary) |
| **Startup** | ~500ms | **~5ms** |
| **Scan Speed** | 1000 files / 10s | **1000 files / 0.2s** |

## 📦 Installation

**No Rust environment required!** The script automatically downloads the pre-compiled binary for your platform.

### Quick Install

```bash
git clone https://github.com/ly87ing/java-perf-skill.git
cd java-perf-skill
./install.sh
```

Supported Platforms:
- macOS Apple Silicon (arm64)
- macOS Intel (x86_64)
- Linux (x86_64)

### Update

```bash
./update.sh
```
*Automatically downloads the latest binary from GitHub Releases.*

### Manual Install (From Source)
*Only if you want to build from scratch:*

```bash
cd rust-mcp
cargo build --release
claude mcp add java-perf --scope user -- $(pwd)/target/release/java-perf
```

## 🔧 MCP Tools (9 Tools)

### 📚 Knowledge Base
| Tool | Description |
|------|-------------|
| `get_checklist` | ❓ Get checklist based on symptoms (memory, cpu, slow...) |
| `get_all_antipatterns` | ⚠️ List all 15+ performance anti-patterns |

### 🛰️ Radar (AST Scan)
| Tool | Description |
|------|-------------|
| `radar_scan` | Project-wide AST scan for performance risks |
| `scan_source_code` | Single file AST analysis |

### 🔬 Forensic (Diagnostics)
| Tool | Description |
|------|-------------|
| `analyze_log` | Log fingerprinting & aggregation |
| `analyze_thread_dump` | `jstack` thread analysis |
| `analyze_bytecode` | `javap` bytecode disassembly |
| `analyze_heap` | `jmap -histo` heap analysis |

### ⚙️ System
| Tool | Description |
|------|-------------|
| `get_engine_status` | Check engine & JDK status |

## 🔍 Detection Rules (17 Rules)

### 🔴 P0 Critical
| ID | Description |
|----|-------------|
| `N_PLUS_ONE` | IO/DB calls inside loops |
| `NESTED_LOOP` | Nested loops O(N*M) |
| `SYNC_METHOD` | Synchronized on method level |
| `THREADLOCAL_LEAK` | ThreadLocal missing .remove() |
| `UNBOUNDED_POOL` | Executors.newCachedThreadPool |
| `UNBOUNDED_CACHE` | static Map without eviction |
| `UNBOUNDED_LIST` | static List/Set growing indefinitely |
| `EMITTER_UNBOUNDED` | Reactor EmitterProcessor (Backpressure) |

### 🟡 P1 Warning
| ID | Description |
|----|-------------|
| `OBJECT_IN_LOOP` | Object allocation inside loops |
| `SYNC_BLOCK` | Large synchronized block |
| `ATOMIC_SPIN` | High contention atomic |
| `NO_TIMEOUT` | HTTP client without timeout |
| `BLOCKING_IO` | Blocking IO in async context |
| `SINKS_NO_BACKPRESSURE` | Sinks.many() without handling |
| `CACHE_NO_EXPIRE` | Cache missing expireAfterWrite |

## 📝 Usage Examples

**Diagnosis:**
> "Help me analyze memory leak issues in this project."

**Scanning:**
> "Scan the whole project for performance risks."

**Forensic:**
> "Analyze this thread dump for deadlocks."

## License

MIT
