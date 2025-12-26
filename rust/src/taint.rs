// ============================================================================
// 污点分析模块 - 跨文件调用链追踪
// ============================================================================

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// 方法签名
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MethodSig {
    pub class: String,      // "UserService"
    pub name: String,       // "getUser"
}

impl MethodSig {
    pub fn new(class: &str, name: &str) -> Self {
        Self {
            class: class.to_string(),
            name: name.to_string(),
        }
    }
    
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.class, self.name)
    }
}

/// 调用点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSite {
    pub file: PathBuf,
    pub line: usize,
    pub callee: MethodSig,
    pub caller: MethodSig,
}

/// 调用图 - 用于追踪 Controller -> Service -> DAO 链
#[derive(Debug, Default)]
pub struct CallGraph {
    /// 方法签名 -> 该方法调用的其他方法
    pub outgoing: HashMap<MethodSig, Vec<CallSite>>,
    /// 方法签名 -> 调用该方法的其他方法  
    pub incoming: HashMap<MethodSig, Vec<CallSite>>,
    /// 类名 -> 文件路径
    pub class_index: HashMap<String, PathBuf>,
    /// 类的 Layer 类型 (Controller/Service/Repository)
    pub class_layers: HashMap<String, LayerType>,
}

/// 代码层级类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    Controller,
    Service,
    Repository,
    Unknown,
}

impl CallGraph {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 添加调用关系
    pub fn add_call(&mut self, caller: MethodSig, callee: MethodSig, file: PathBuf, line: usize) {
        let call_site = CallSite {
            file: file.clone(),
            line,
            callee: callee.clone(),
            caller: caller.clone(),
        };
        
        // 添加出边
        self.outgoing
            .entry(caller.clone())
            .or_default()
            .push(call_site.clone());
        
        // 添加入边
        self.incoming
            .entry(callee)
            .or_default()
            .push(call_site);
    }
    
    /// 注册类信息
    pub fn register_class(&mut self, class_name: &str, file: PathBuf, layer: LayerType) {
        self.class_index.insert(class_name.to_string(), file);
        self.class_layers.insert(class_name.to_string(), layer);
    }
    
    /// 追踪从某个方法到目标层的路径
    /// 例如：从 Controller 方法追踪到 Repository 方法
    pub fn trace_to_layer(&self, start: &MethodSig, target_layer: LayerType, max_depth: usize) -> Vec<Vec<MethodSig>> {
        let mut paths = Vec::new();
        let mut current_path = vec![start.clone()];
        let mut visited = std::collections::HashSet::new();
        
        self.dfs_trace(start, target_layer, max_depth, &mut current_path, &mut visited, &mut paths);
        
        paths
    }
    
    fn dfs_trace(
        &self,
        current: &MethodSig,
        target_layer: LayerType,
        remaining_depth: usize,
        path: &mut Vec<MethodSig>,
        visited: &mut std::collections::HashSet<MethodSig>,
        result: &mut Vec<Vec<MethodSig>>,
    ) {
        if remaining_depth == 0 {
            return;
        }
        
        // 检查当前方法是否在目标层
        if let Some(layer) = self.class_layers.get(&current.class) {
            if *layer == target_layer && path.len() > 1 {
                result.push(path.clone());
                return;
            }
        }
        
        // 继续 DFS
        if let Some(callees) = self.outgoing.get(current) {
            for call_site in callees {
                if !visited.contains(&call_site.callee) {
                    visited.insert(call_site.callee.clone());
                    path.push(call_site.callee.clone());
                    
                    self.dfs_trace(&call_site.callee, target_layer, remaining_depth - 1, path, visited, result);
                    
                    path.pop();
                    visited.remove(&call_site.callee);
                }
            }
        }
    }
    
    /// 检测 N+1 问题：在循环内调用的方法最终是否到达 Repository
    pub fn detect_n_plus_one_chains(&self) -> Vec<CallChainReport> {
        let mut reports = Vec::new();
        
        // 查找所有 Repository 方法
        for (method, incoming_calls) in &self.incoming {
            if let Some(layer) = self.class_layers.get(&method.class) {
                if *layer == LayerType::Repository {
                    // 对每个调用点，追踪回到 Controller
                    for call_site in incoming_calls {
                        let paths = self.trace_to_layer(&call_site.caller, LayerType::Controller, 5);
                        if !paths.is_empty() {
                            reports.push(CallChainReport {
                                dao_method: method.clone(),
                                call_site: call_site.clone(),
                                controller_paths: paths,
                            });
                        }
                    }
                }
            }
        }
        
        reports
    }
}

/// 调用链报告
#[derive(Debug, Serialize)]
pub struct CallChainReport {
    pub dao_method: MethodSig,
    pub call_site: CallSite,
    pub controller_paths: Vec<Vec<MethodSig>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_call_graph_basic() {
        let mut graph = CallGraph::new();
        
        // 注册类
        graph.register_class("UserController", PathBuf::from("UserController.java"), LayerType::Controller);
        graph.register_class("UserService", PathBuf::from("UserService.java"), LayerType::Service);
        graph.register_class("UserRepository", PathBuf::from("UserRepository.java"), LayerType::Repository);
        
        // Controller -> Service
        graph.add_call(
            MethodSig::new("UserController", "getUsers"),
            MethodSig::new("UserService", "findAll"),
            PathBuf::from("UserController.java"),
            10,
        );
        
        // Service -> Repository
        graph.add_call(
            MethodSig::new("UserService", "findAll"),
            MethodSig::new("UserRepository", "findById"),
            PathBuf::from("UserService.java"),
            20,
        );
        
        // 追踪 Controller -> Repository
        let paths = graph.trace_to_layer(
            &MethodSig::new("UserController", "getUsers"),
            LayerType::Repository,
            5,
        );
        
        assert!(!paths.is_empty(), "Should find path from Controller to Repository");
        assert_eq!(paths[0].len(), 3); // Controller -> Service -> Repository
    }
}
