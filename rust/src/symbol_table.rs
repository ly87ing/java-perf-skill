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

/// 方法参数信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamInfo {
    pub name: String,
    pub type_name: String,
}

/// 方法信息 (v9.2: 增强版，支持参数签名)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub class: String,
    pub return_type: Option<String>,
    pub params: Vec<ParamInfo>,  // v9.2: 参数列表
    pub annotations: Vec<String>,
    pub line: usize,
}

impl MethodInfo {
    pub fn new(name: &str, class: &str, line: usize) -> Self {
        Self {
            name: name.to_string(),
            class: class.to_string(),
            return_type: None,
            params: Vec::new(),
            annotations: Vec::new(),
            line,
        }
    }

    /// 生成方法签名 (用于区分重载)
    pub fn signature(&self) -> String {
        let param_types: Vec<&str> = self.params.iter()
            .map(|p| p.type_name.as_str())
            .collect();
        format!("{}({})", self.name, param_types.join(","))
    }

    /// 添加参数
    pub fn add_param(&mut self, name: &str, type_name: &str) {
        self.params.push(ParamInfo {
            name: name.to_string(),
            type_name: type_name.to_string(),
        });
    }
}

/// 符号表 - 跟踪类型和变量 (v9.2: 支持方法重载)
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// 类名 -> 类型信息
    pub classes: HashMap<String, TypeInfo>,
    /// (类名, 字段名) -> 变量绑定
    pub fields: HashMap<(String, String), VarBinding>,
    /// (类名, 方法签名) -> 方法信息
    /// 注意: 方法签名格式为 "methodName(Type1,Type2)"
    pub methods: HashMap<(String, String), MethodInfo>,
    /// (类名, 方法名) -> 方法签名列表 (用于查找重载)
    method_index: HashMap<(String, String), Vec<String>>,
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

    /// 注册方法 (v9.2: 支持重载)
    pub fn register_method(&mut self, class: &str, info: MethodInfo) {
        let sig = info.signature();
        let name = info.name.clone();

        // 添加到方法表
        self.methods.insert((class.to_string(), sig.clone()), info);

        // 更新方法索引
        self.method_index
            .entry((class.to_string(), name))
            .or_default()
            .push(sig);
    }

    /// 查找方法 (按名称，返回所有重载)
    pub fn lookup_methods(&self, class: &str, method_name: &str) -> Vec<&MethodInfo> {
        if let Some(signatures) = self.method_index.get(&(class.to_string(), method_name.to_string())) {
            signatures.iter()
                .filter_map(|sig| self.methods.get(&(class.to_string(), sig.clone())))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 查找方法 (按签名，精确匹配)
    pub fn lookup_method_by_sig(&self, class: &str, signature: &str) -> Option<&MethodInfo> {
        self.methods.get(&(class.to_string(), signature.to_string()))
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

    #[test]
    fn test_method_overload() {
        let mut table = SymbolTable::new();

        // 注册两个重载方法
        let mut find_by_id = MethodInfo::new("find", "UserRepository", 10);
        find_by_id.add_param("id", "Long");
        find_by_id.return_type = Some("User".to_string());

        let mut find_by_name = MethodInfo::new("find", "UserRepository", 15);
        find_by_name.add_param("name", "String");
        find_by_name.return_type = Some("User".to_string());

        table.register_method("UserRepository", find_by_id);
        table.register_method("UserRepository", find_by_name);

        // 按名称查找应返回两个方法
        let methods = table.lookup_methods("UserRepository", "find");
        assert_eq!(methods.len(), 2);

        // 按签名查找应返回精确匹配
        let method = table.lookup_method_by_sig("UserRepository", "find(Long)");
        assert!(method.is_some());
        assert_eq!(method.unwrap().params[0].type_name, "Long");

        let method2 = table.lookup_method_by_sig("UserRepository", "find(String)");
        assert!(method2.is_some());
        assert_eq!(method2.unwrap().params[0].type_name, "String");
    }

    #[test]
    fn test_method_signature() {
        let mut method = MethodInfo::new("save", "UserRepository", 20);
        method.add_param("user", "User");
        method.add_param("flush", "boolean");

        assert_eq!(method.signature(), "save(User,boolean)");
    }
}
