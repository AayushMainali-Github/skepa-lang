use crate::types::TypeInfo;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IrType {
    Int,
    Float,
    Bool,
    String,
    Bytes,
    Void,
    Option {
        value: Box<IrType>,
    },
    Result {
        ok: Box<IrType>,
        err: Box<IrType>,
    },
    Named(String),
    Opaque(String),
    Array {
        elem: Box<IrType>,
        size: usize,
    },
    Vec {
        elem: Box<IrType>,
    },
    Map {
        value: Box<IrType>,
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
            TypeInfo::Bytes => Self::Bytes,
            TypeInfo::Void => Self::Void,
            TypeInfo::Option { value } => Self::Option {
                value: Box::new(Self::from(value.as_ref())),
            },
            TypeInfo::Result { ok, err } => Self::Result {
                ok: Box::new(Self::from(ok.as_ref())),
                err: Box::new(Self::from(err.as_ref())),
            },
            TypeInfo::Named(name) => Self::Named(name.clone()),
            TypeInfo::Opaque(name) => Self::Opaque(name.clone()),
            TypeInfo::Array { elem, size } => Self::Array {
                elem: Box::new(Self::from(elem.as_ref())),
                size: *size,
            },
            TypeInfo::Vec { elem } => Self::Vec {
                elem: Box::new(Self::from(elem.as_ref())),
            },
            TypeInfo::Map { value } => Self::Map {
                value: Box::new(Self::from(value.as_ref())),
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
