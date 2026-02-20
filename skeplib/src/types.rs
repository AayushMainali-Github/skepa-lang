use crate::ast::TypeName;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeInfo {
    Int,
    Float,
    Bool,
    String,
    Void,
    Named(String),
    Array {
        elem: Box<TypeInfo>,
        size: usize,
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
            TypeName::Void => TypeInfo::Void,
            TypeName::Named(name) => TypeInfo::Named(name.clone()),
            TypeName::Array { elem, size } => TypeInfo::Array {
                elem: Box::new(TypeInfo::from_ast(elem)),
                size: *size,
            },
            TypeName::Fn { params, ret } => TypeInfo::Fn {
                params: params.iter().map(TypeInfo::from_ast).collect(),
                ret: Box::new(TypeInfo::from_ast(ret)),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<TypeInfo>,
    pub ret: TypeInfo,
}
