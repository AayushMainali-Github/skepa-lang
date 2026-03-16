use crate::types::TypeInfo;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IrType {
    Int,
    Float,
    Bool,
    String,
    Void,
    Named(String),
    Array {
        elem: Box<IrType>,
        size: usize,
    },
    Vec {
        elem: Box<IrType>,
    },
    Fn {
        params: Vec<IrType>,
        ret: Box<IrType>,
    },
    Unknown,
}

impl From<&TypeInfo> for IrType {
    fn from(value: &TypeInfo) -> Self {
        match value {
            TypeInfo::Int => Self::Int,
            TypeInfo::Float => Self::Float,
            TypeInfo::Bool => Self::Bool,
            TypeInfo::String => Self::String,
            TypeInfo::Void => Self::Void,
            TypeInfo::Named(name) => Self::Named(name.clone()),
            TypeInfo::Array { elem, size } => Self::Array {
                elem: Box::new(Self::from(elem.as_ref())),
                size: *size,
            },
            TypeInfo::Vec { elem } => Self::Vec {
                elem: Box::new(Self::from(elem.as_ref())),
            },
            TypeInfo::Fn { params, ret } => Self::Fn {
                params: params.iter().map(Self::from).collect(),
                ret: Box::new(Self::from(ret.as_ref())),
            },
            TypeInfo::Unknown => Self::Unknown,
        }
    }
}

impl IrType {
    pub fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }
}
