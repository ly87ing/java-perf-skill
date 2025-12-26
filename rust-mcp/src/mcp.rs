//! MCP Protocol Handler
//! 
//! 处理 JSON-RPC 2.0 请求/响应

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::{ast_engine, forensic, jdk_engine};

/// JSON-RPC 请求
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Value,
}

/// JSON-RPC 响应
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

/// MCP 工具定义
fn get_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "radar_scan",
                "description": "🛰️ 雷达扫描 - 全项目 AST 分析，返回嫌疑点列表 (P0/P1)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "codePath": {
                            "type": "string",
                            "description": "项目根路径"
                        }
                    },
                    "required": ["codePath"]
                }
            },
            {
                "name": "scan_source_code",
                "description": "🛰️ 单文件 AST 分析",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "code": { "type": "string", "description": "源代码内容" },
                        "filePath": { "type": "string", "description": "文件路径" }
                    },
                    "required": ["code"]
                }
            },
            {
                "name": "analyze_log",
                "description": "🔬 日志指纹归类分析",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "logPath": { "type": "string", "description": "日志文件路径" }
                    },
                    "required": ["logPath"]
                }
            },
            {
                "name": "analyze_thread_dump",
                "description": "🔬 线程 Dump 分析 (jstack)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pid": { "type": "integer", "description": "Java 进程 PID" }
                    },
                    "required": ["pid"]
                }
            },
            {
                "name": "get_engine_status",
                "description": "获取引擎状态",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}

/// 处理 MCP 请求
pub fn handle_request(request: &str) -> Result<String, Box<dyn std::error::Error>> {
    let req: JsonRpcRequest = serde_json::from_str(request)?;
    
    let result = match req.method.as_str() {
        // MCP 协议方法
        "initialize" => handle_initialize(&req.params),
        "notifications/initialized" => return Ok(String::new()), // 无响应
        "tools/list" => Ok(get_tools()),
        "tools/call" => handle_tool_call(&req.params),
        
        // 未知方法
        _ => Err(format!("Unknown method: {}", req.method).into()),
    };
    
    let response = match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(value),
            error: None,
            id: req.id,
        },
        Err(e) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32603,
                message: e.to_string(),
            }),
            id: req.id,
        },
    };
    
    Ok(serde_json::to_string(&response)?)
}

/// 创建错误响应
pub fn create_error_response(request: &str, error: &str) -> String {
    let id = serde_json::from_str::<JsonRpcRequest>(request)
        .map(|r| r.id)
        .unwrap_or(Value::Null);
    
    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(JsonRpcError {
            code: -32603,
            message: error.to_string(),
        }),
        id,
    };
    
    serde_json::to_string(&response).unwrap_or_default()
}

/// 处理 initialize
fn handle_initialize(_params: &Option<Value>) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "java-perf",
            "version": "4.0.0"
        }
    }))
}

/// 处理工具调用
fn handle_tool_call(params: &Option<Value>) -> Result<Value, Box<dyn std::error::Error>> {
    let params = params.as_ref().ok_or("Missing params")?;
    let tool_name = params.get("name").and_then(|v| v.as_str()).ok_or("Missing tool name")?;
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
    
    let result = match tool_name {
        "radar_scan" => {
            let code_path = arguments.get("codePath")
                .and_then(|v| v.as_str())
                .unwrap_or("./");
            ast_engine::radar_scan(code_path)
        },
        "scan_source_code" => {
            let code = arguments.get("code")
                .and_then(|v| v.as_str())
                .ok_or("Missing code")?;
            let file_path = arguments.get("filePath")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown.java");
            ast_engine::scan_source_code(code, file_path)
        },
        "analyze_log" => {
            let log_path = arguments.get("logPath")
                .and_then(|v| v.as_str())
                .ok_or("Missing logPath")?;
            forensic::analyze_log(log_path)
        },
        "analyze_thread_dump" => {
            let pid = arguments.get("pid")
                .and_then(|v| v.as_i64())
                .ok_or("Missing pid")? as u32;
            jdk_engine::analyze_thread_dump(pid)
        },
        "get_engine_status" => {
            Ok(json!({
                "version": "4.0.0",
                "engine": "Rust Radar-Sniper",
                "ast": "tree-sitter-java",
                "jdk": jdk_engine::check_jdk_available()
            }))
        },
        _ => Err(format!("Unknown tool: {}", tool_name).into()),
    };
    
    match result {
        Ok(content) => Ok(json!({
            "content": [{
                "type": "text",
                "text": content.to_string()
            }]
        })),
        Err(e) => Ok(json!({
            "content": [{
                "type": "text",
                "text": format!("Error: {}", e)
            }],
            "isError": true
        })),
    }
}
