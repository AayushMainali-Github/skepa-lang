use crate::ast::TypeName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeInfo {
    Int,
    Float,
    Bool,
    String,
    Void,
    Unknown,
}

impl TypeInfo {
    pub fn from_ast(ty: TypeName) -> Self {
        match ty {
            TypeName::Int => TypeInfo::Int,
            TypeName::Float => TypeInfo::Float,
            TypeName::Bool => TypeInfo::Bool,
            TypeName::String => TypeInfo::String,
            TypeName::Void => TypeInfo::Void,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<TypeInfo>,
    pub ret: TypeInfo,
}
