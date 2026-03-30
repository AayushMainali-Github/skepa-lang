use crate::ast::TypeName;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeInfo {
    Int,
    Float,
    Bool,
    String,
    Bytes,
    Void,
    Named(String),
    Opaque(String),
    Array {
        elem: Box<TypeInfo>,
        size: usize,
    },
    Vec {
        elem: Box<TypeInfo>,
    },
    Map {
        value: Box<TypeInfo>,
    },
    Fn {
        params: Vec<TypeInfo>,
        ret: Box<TypeInfo>,
    },
    Unknown,
}

impl TypeInfo {
    pub fn from_ast(ty: &TypeName) -> Self {
        match ty {
            TypeName::Int => TypeInfo::Int,
            TypeName::Float => TypeInfo::Float,
            TypeName::Bool => TypeInfo::Bool,
            TypeName::String => TypeInfo::String,
            TypeName::Bytes => TypeInfo::Bytes,
            TypeName::Void => TypeInfo::Void,
            TypeName::Named(name) => {
                if is_builtin_opaque_type(name) {
                    TypeInfo::Opaque(name.clone())
                } else {
                    TypeInfo::Named(name.clone())
                }
            }
            TypeName::Array { elem, size } => TypeInfo::Array {
                elem: Box::new(TypeInfo::from_ast(elem)),
                size: *size,
            },
            TypeName::Vec { elem } => TypeInfo::Vec {
                elem: Box::new(TypeInfo::from_ast(elem)),
            },
            TypeName::Map { value } => TypeInfo::Map {
                value: Box::new(TypeInfo::from_ast(value)),
            },
            TypeName::Fn { params, ret } => TypeInfo::Fn {
                params: params.iter().map(TypeInfo::from_ast).collect(),
                ret: Box::new(TypeInfo::from_ast(ret)),
            },
        }
    }
}

pub fn task_channel_type_name(value: &TypeInfo) -> String {
    format!("task.Channel[{}]", display_type(value))
}

pub fn task_task_type_name(value: &TypeInfo) -> String {
    format!("task.Task[{}]", display_type(value))
}

pub fn task_channel_type(value: &TypeInfo) -> TypeInfo {
    TypeInfo::Opaque(task_channel_type_name(value))
}

pub fn task_task_type(value: &TypeInfo) -> TypeInfo {
    TypeInfo::Opaque(task_task_type_name(value))
}

pub fn task_channel_value_type(name: &str) -> Option<TypeInfo> {
    let inner = name.strip_prefix("task.Channel[")?.strip_suffix(']')?;
    parse_display_type(inner)
}

pub fn task_task_value_type(name: &str) -> Option<TypeInfo> {
    let inner = name.strip_prefix("task.Task[")?.strip_suffix(']')?;
    parse_display_type(inner)
}

fn display_type(value: &TypeInfo) -> String {
    match value {
        TypeInfo::Int => "Int".to_string(),
        TypeInfo::Float => "Float".to_string(),
        TypeInfo::Bool => "Bool".to_string(),
        TypeInfo::String => "String".to_string(),
        TypeInfo::Bytes => "Bytes".to_string(),
        TypeInfo::Void => "Void".to_string(),
        TypeInfo::Named(name) | TypeInfo::Opaque(name) => name.clone(),
        TypeInfo::Array { elem, size } => format!("[{}; {}]", display_type(elem), size),
        TypeInfo::Vec { elem } => format!("Vec[{}]", display_type(elem)),
        TypeInfo::Map { value } => format!("Map[String, {}]", display_type(value)),
        TypeInfo::Fn { params, ret } => format!(
            "Fn({}) -> {}",
            params
                .iter()
                .map(display_type)
                .collect::<Vec<_>>()
                .join(", "),
            display_type(ret)
        ),
        TypeInfo::Unknown => "Unknown".to_string(),
    }
}

fn parse_display_type(value: &str) -> Option<TypeInfo> {
    match value {
        "Int" => Some(TypeInfo::Int),
        "Float" => Some(TypeInfo::Float),
        "Bool" => Some(TypeInfo::Bool),
        "String" => Some(TypeInfo::String),
        "Bytes" => Some(TypeInfo::Bytes),
        "Void" => Some(TypeInfo::Void),
        _ => {
            if let Some(inner) = value.strip_prefix("Vec[").and_then(|v| v.strip_suffix(']')) {
                return Some(TypeInfo::Vec {
                    elem: Box::new(parse_display_type(inner)?),
                });
            }
            if let Some(inner) = value
                .strip_prefix("Map[String, ")
                .and_then(|v| v.strip_suffix(']'))
            {
                return Some(TypeInfo::Map {
                    value: Box::new(parse_display_type(inner)?),
                });
            }
            if (value.starts_with("task.Channel[") || value.starts_with("task.Task["))
                && value.ends_with(']')
            {
                return Some(TypeInfo::Opaque(value.to_string()));
            }
            Some(TypeInfo::Named(value.to_string()))
        }
    }
}

pub fn is_builtin_opaque_type(name: &str) -> bool {
    matches!(
        name,
        "net.Socket" | "net.Listener" | "task.Task" | "task.Channel"
    ) || ((name.starts_with("task.Channel[") || name.starts_with("task.Task["))
        && name.ends_with(']'))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<TypeInfo>,
    pub ret: TypeInfo,
}
