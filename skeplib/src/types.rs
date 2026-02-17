use crate::ast::TypeName;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeInfo {
    Int,
    Float,
    Bool,
    String,
    Void,
    Array { elem: Box<TypeInfo>, size: usize },
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
            TypeName::Named(_) => TypeInfo::Unknown,
            TypeName::Array { elem, size } => TypeInfo::Array {
                elem: Box::new(TypeInfo::from_ast(elem)),
                size: *size,
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
