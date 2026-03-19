use std::rc::Rc;

use crate::{RtArray, RtError, RtResult, RtString, RtVec};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RtFunctionRef(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtStructLayout {
    pub name: String,
    pub field_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RtStruct {
    pub layout: Rc<RtStructLayout>,
    pub fields: Vec<RtValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RtValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(RtString),
    Array(RtArray),
    Vec(RtVec),
    Function(RtFunctionRef),
    Struct(RtStruct),
    Unit,
}

impl RtValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Int(_) => "Int",
            Self::Float(_) => "Float",
            Self::Bool(_) => "Bool",
            Self::String(_) => "String",
            Self::Array(_) => "Array",
            Self::Vec(_) => "Vec",
            Self::Function(_) => "Function",
            Self::Struct(_) => "Struct",
            Self::Unit => "Void",
        }
    }

    pub fn expect_int(&self) -> RtResult<i64> {
        match self {
            Self::Int(value) => Ok(*value),
            other => Err(RtError::type_mismatch(format!(
                "expected Int, got {}",
                other.type_name()
            ))),
        }
    }

    pub fn expect_float(&self) -> RtResult<f64> {
        match self {
            Self::Float(value) => Ok(*value),
            other => Err(RtError::type_mismatch(format!(
                "expected Float, got {}",
                other.type_name()
            ))),
        }
    }

    pub fn expect_bool(&self) -> RtResult<bool> {
        match self {
            Self::Bool(value) => Ok(*value),
            other => Err(RtError::type_mismatch(format!(
                "expected Bool, got {}",
                other.type_name()
            ))),
        }
    }

    pub fn expect_string(&self) -> RtResult<RtString> {
        match self {
            Self::String(value) => Ok(value.clone()),
            other => Err(RtError::type_mismatch(format!(
                "expected String, got {}",
                other.type_name()
            ))),
        }
    }

    pub fn expect_array(&self) -> RtResult<RtArray> {
        match self {
            Self::Array(value) => Ok(value.clone()),
            other => Err(RtError::type_mismatch(format!(
                "expected Array, got {}",
                other.type_name()
            ))),
        }
    }

    pub fn expect_vec(&self) -> RtResult<RtVec> {
        match self {
            Self::Vec(value) => Ok(value.clone()),
            other => Err(RtError::type_mismatch(format!(
                "expected Vec, got {}",
                other.type_name()
            ))),
        }
    }

    pub fn expect_struct(&self) -> RtResult<RtStruct> {
        match self {
            Self::Struct(value) => Ok(value.clone()),
            other => Err(RtError::type_mismatch(format!(
                "expected Struct, got {}",
                other.type_name()
            ))),
        }
    }

    pub fn expect_function(&self) -> RtResult<RtFunctionRef> {
        match self {
            Self::Function(value) => Ok(*value),
            other => Err(RtError::type_mismatch(format!(
                "expected Function, got {}",
                other.type_name()
            ))),
        }
    }
}

impl RtStruct {
    pub fn new(layout: Rc<RtStructLayout>, fields: Vec<RtValue>) -> RtResult<Self> {
        if !layout.field_names.is_empty() && layout.field_names.len() != fields.len() {
            return Err(RtError::new(
                crate::RtErrorKind::MissingField,
                format!(
                    "struct `{}` expected {} fields, got {}",
                    layout.name,
                    layout.field_names.len(),
                    fields.len()
                ),
            ));
        }
        Ok(Self { layout, fields })
    }

    pub fn named(name: impl Into<String>, fields: Vec<RtValue>) -> RtResult<Self> {
        Self::new(
            Rc::new(RtStructLayout {
                name: name.into(),
                field_names: Vec::new(),
            }),
            fields,
        )
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.layout
            .field_names
            .iter()
            .position(|field| field == name)
    }

    pub fn get_field(&self, index: usize) -> RtResult<RtValue> {
        self.fields
            .get(index)
            .cloned()
            .ok_or_else(|| RtError::new(crate::RtErrorKind::MissingField, "field out of range"))
    }

    pub fn set_field(&mut self, index: usize, value: RtValue) -> RtResult<()> {
        let slot = self
            .fields
            .get_mut(index)
            .ok_or_else(|| RtError::new(crate::RtErrorKind::MissingField, "field out of range"))?;
        *slot = value;
        Ok(())
    }

    pub fn get_named_field(&self, name: &str) -> RtResult<RtValue> {
        let index = self.field_index(name).ok_or_else(|| {
            RtError::new(
                crate::RtErrorKind::MissingField,
                format!("unknown field `{name}` on `{}`", self.layout.name),
            )
        })?;
        self.get_field(index)
    }
}
