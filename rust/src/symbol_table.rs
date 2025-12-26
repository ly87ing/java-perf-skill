// ============================================================================
// 符号表模块 - 轻量级类型追踪
// ============================================================================

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// 代码层级类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    Controller,
    Service,
    Repository,
    Component,
    Unknown,
}

impl LayerType {
    /// 从注解名称推断层级
    pub fn from_annotation(annotation: &str) -> Self {
        match annotation {
            "Controller" | "RestController" => LayerType::Controller,
            "Service" => LayerType::Service,
            "Repository" | "Mapper" => LayerType::Repository,
            "Component" => LayerType::Component,
            _ => LayerType::Unknown,
        }
    }
    
    /// 判断是否可能进行 DB/RPC 操作
    pub fn is_io_layer(&self) -> bool {
        matches!(self, LayerType::Repository)
    }
}

/// 类型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,               // "UserRepository"
    pub package: Option<String>,    // "com.example.repository"
    pub annotations: Vec<String>,   // ["Repository", "Component"]
    pub layer: LayerType,
    pub file: PathBuf,
    pub line: usize,
}

impl TypeInfo {
    pub fn new(name: &str, file: PathBuf, line: usize) -> Self {
        Self {
            name: name.to_string(),
            package: None,
            annotations: Vec::new(),
            layer: LayerType::Unknown,
            file,
            line,
        }
    }
    
    /// 添加注解并更新层级
    pub fn add_annotation(&mut self, annotation: &str) {
        self.annotations.push(annotation.to_string());
        // 更新层级（取优先级最高的）
        let new_layer = LayerType::from_annotation(annotation);
        if new_layer != LayerType::Unknown {
            self.layer = new_layer;
        }
    }
    
    /// 判断是否是 DAO 类型
    pub fn is_dao(&self) -> bool {
        self.layer == LayerType::Repository
            || self.annotations.iter().any(|a| {
                a == "Repository" || a == "Mapper" || a.ends_with("Repository") || a.ends_with("Dao")
            })
            || self.name.ends_with("Repository")
            || self.name.ends_with("Dao")
            || self.name.ends_with("Mapper")
    }
}

/// 变量绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarBinding {
    pub name: String,           // "userRepository"
    pub type_name: String,      // "UserRepository"
    pub is_field: bool,         // 是否是字段（而非局部变量）
    pub annotations: Vec<String>, // 字段上的注解，如 ["Autowired"]
}

impl VarBinding {
    pub fn new(name: &str, type_name: &str, is_field: bool) -> Self {
        Self {
            name: name.to_string(),
            type_name: type_name.to_string(),
            is_field,
            annotations: Vec::new(),
        }
    }
}

/// 方法信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub class: String,
    pub return_type: Option<String>,
    pub annotations: Vec<String>,
    pub line: usize,
}

/// 符号表 - 跟踪类型和变量
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// 类名 -> 类型信息
    pub classes: HashMap<String, TypeInfo>,
    /// (类名, 字段名) -> 变量绑定
    pub fields: HashMap<(String, String), VarBinding>,
    /// (类名, 方法名) -> 方法信息
    pub methods: HashMap<(String, String), MethodInfo>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 注册类
    pub fn register_class(&mut self, info: TypeInfo) {
        self.classes.insert(info.name.clone(), info);
    }
    
    /// 注册字段
    pub fn register_field(&mut self, class: &str, binding: VarBinding) {
        self.fields.insert((class.to_string(), binding.name.clone()), binding);
    }
    
    /// 注册方法
    pub fn register_method(&mut self, info: MethodInfo) {
        self.methods.insert((info.class.clone(), info.name.clone()), info);
    }
    
    /// 查询变量的类型信息
    pub fn lookup_var_type(&self, class: &str, var_name: &str) -> Option<&TypeInfo> {
        // 先查字段
        if let Some(binding) = self.fields.get(&(class.to_string(), var_name.to_string())) {
            return self.classes.get(&binding.type_name);
        }
        None
    }
    
    /// 判断变量是否是 DAO 类型
    pub fn is_dao_var(&self, class: &str, var_name: &str) -> bool {
        if let Some(type_info) = self.lookup_var_type(class, var_name) {
            return type_info.is_dao();
        }
        // 退化到名称猜测
        var_name.ends_with("Repository") 
            || var_name.ends_with("Dao") 
            || var_name.ends_with("Mapper")
            || var_name.contains("repository")
            || var_name.contains("dao")
    }
    
    /// 判断方法调用是否是 DAO 操作
    pub fn is_dao_call(&self, class: &str, receiver: &str, method: &str) -> bool {
        // 1. 检查接收者类型
        if self.is_dao_var(class, receiver) {
            return true;
        }
        
        // 2. 检查方法名模式（DAO 常见方法）
        let dao_methods = [
            "find", "save", "delete", "update", "insert", "select",
            "getById", "findById", "findAll", "findOne",
            "saveAll", "deleteById", "deleteAll",
            "execute", "query", "count",
        ];
        
        for pattern in dao_methods {
            if method.starts_with(pattern) || method.contains(pattern) {
                return true;
            }
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_layer_from_annotation() {
        assert_eq!(LayerType::from_annotation("Repository"), LayerType::Repository);
        assert_eq!(LayerType::from_annotation("RestController"), LayerType::Controller);
        assert_eq!(LayerType::from_annotation("Service"), LayerType::Service);
    }
    
    #[test]
    fn test_is_dao_type() {
        let mut type_info = TypeInfo::new("UserRepository", PathBuf::from("test.java"), 1);
        assert!(type_info.is_dao()); // 基于名称
        
        type_info.add_annotation("Repository");
        assert!(type_info.is_dao()); // 基于注解
        assert_eq!(type_info.layer, LayerType::Repository);
    }
    
    #[test]
    fn test_symbol_table_lookup() {
        let mut table = SymbolTable::new();
        
        // 注册 Repository 类
        let mut repo_type = TypeInfo::new("UserRepository", PathBuf::from("UserRepository.java"), 1);
        repo_type.add_annotation("Repository");
        table.register_class(repo_type);
        
        // 注册字段
        let binding = VarBinding::new("userRepo", "UserRepository", true);
        table.register_field("UserService", binding);
        
        // 测试查询
        assert!(table.is_dao_var("UserService", "userRepo"));
        assert!(table.is_dao_call("UserService", "userRepo", "findById"));
    }
}
