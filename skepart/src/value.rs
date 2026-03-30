use std::rc::Rc;

use crate::{RtArray, RtBytes, RtError, RtMap, RtResult, RtString, RtVec};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RtFunctionRef(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RtHandleKind {
    Socket,
    Listener,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RtHandle {
    pub id: usize,
    pub kind: RtHandleKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtStructLayout {
    pub name: String,
    pub field_names: Vec<String>,
    pub field_types: Vec<Option<&'static str>>,
}

#[derive(Debug, Clone, PartialEq)]
enum RtStructFields {
    Values(Vec<RtValue>),
    Ints(Vec<i64>),
    Floats(Vec<f64>),
    Bools(Vec<bool>),
    Strings(Vec<RtString>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RtStruct {
    pub layout: Rc<RtStructLayout>,
    fields: RtStructFields,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RtValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(RtString),
    Bytes(RtBytes),
    Array(RtArray),
    Vec(RtVec),
    Map(RtMap),
    Function(RtFunctionRef),
    Handle(RtHandle),
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
            Self::Bytes(_) => "Bytes",
            Self::Array(_) => "Array",
            Self::Vec(_) => "Vec",
            Self::Map(_) => "Map",
            Self::Function(_) => "Function",
            Self::Handle(_) => "Handle",
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

    pub fn expect_bytes(&self) -> RtResult<RtBytes> {
        match self {
            Self::Bytes(value) => Ok(value.clone()),
            other => Err(RtError::type_mismatch(format!(
                "expected Bytes, got {}",
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

    pub fn expect_map(&self) -> RtResult<RtMap> {
        match self {
            Self::Map(value) => Ok(value.clone()),
            other => Err(RtError::type_mismatch(format!(
                "expected Map, got {}",
                other.type_name()
            ))),
        }
    }

    pub fn expect_string_vec(&self) -> RtResult<Vec<String>> {
        let value = self.expect_vec()?;
        (0..value.len())
            .map(|index| {
                value
                    .get(index)?
                    .expect_string()
                    .map(|item| item.as_str().to_owned())
            })
            .collect()
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

    pub fn expect_handle(&self) -> RtResult<RtHandle> {
        match self {
            Self::Handle(value) => Ok(*value),
            other => Err(RtError::type_mismatch(format!(
                "expected Handle, got {}",
                other.type_name()
            ))),
        }
    }

    pub fn expect_handle_kind(&self, kind: RtHandleKind) -> RtResult<RtHandle> {
        let handle = self.expect_handle()?;
        if handle.kind != kind {
            return Err(RtError::invalid_handle_kind(
                kind.type_name(),
                handle.kind.type_name(),
            ));
        }
        Ok(handle)
    }
}

impl RtHandleKind {
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Socket => "Socket",
            Self::Listener => "Listener",
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
        if !layout.field_types.is_empty() && layout.field_types.len() != fields.len() {
            return Err(RtError::new(
                crate::RtErrorKind::MissingField,
                format!(
                    "struct `{}` expected {} typed fields, got {}",
                    layout.name,
                    layout.field_types.len(),
                    fields.len()
                ),
            ));
        }
        for (index, (field, expected)) in fields.iter().zip(&layout.field_types).enumerate() {
            if let Some(expected) = expected {
                if field.type_name() == *expected {
                    continue;
                }
                return Err(RtError::type_mismatch(format!(
                    "struct `{}` field {} expected {}, got {}",
                    layout.name,
                    index,
                    expected,
                    field.type_name()
                )));
            }
        }
        Ok(Self {
            layout,
            fields: Self::infer_fields(fields),
        })
    }

    pub fn named(name: impl Into<String>, fields: Vec<RtValue>) -> RtResult<Self> {
        Self::new(
            Rc::new(RtStructLayout {
                name: name.into(),
                field_names: Vec::new(),
                field_types: Vec::new(),
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
        match &self.fields {
            RtStructFields::Values(fields) => fields.get(index).cloned(),
            RtStructFields::Ints(fields) => fields.get(index).copied().map(RtValue::Int),
            RtStructFields::Floats(fields) => fields.get(index).copied().map(RtValue::Float),
            RtStructFields::Bools(fields) => fields.get(index).copied().map(RtValue::Bool),
            RtStructFields::Strings(fields) => fields.get(index).cloned().map(RtValue::String),
        }
        .ok_or_else(|| RtError::new(crate::RtErrorKind::MissingField, "field out of range"))
    }

    pub fn set_field(&mut self, index: usize, value: RtValue) -> RtResult<()> {
        if let Some(Some(expected)) = self.layout.field_types.get(index) {
            if value.type_name() != *expected {
                return Err(RtError::type_mismatch(format!(
                    "struct `{}` field {} expected {}, got {}",
                    self.layout.name,
                    index,
                    expected,
                    value.type_name()
                )));
            }
        }
        match (&mut self.fields, value) {
            (RtStructFields::Values(fields), value) => {
                let slot = fields.get_mut(index).ok_or_else(|| {
                    RtError::new(crate::RtErrorKind::MissingField, "field out of range")
                })?;
                *slot = value;
                Ok(())
            }
            (RtStructFields::Ints(fields), RtValue::Int(value)) => {
                let slot = fields.get_mut(index).ok_or_else(|| {
                    RtError::new(crate::RtErrorKind::MissingField, "field out of range")
                })?;
                *slot = value;
                Ok(())
            }
            (RtStructFields::Floats(fields), RtValue::Float(value)) => {
                let slot = fields.get_mut(index).ok_or_else(|| {
                    RtError::new(crate::RtErrorKind::MissingField, "field out of range")
                })?;
                *slot = value;
                Ok(())
            }
            (RtStructFields::Bools(fields), RtValue::Bool(value)) => {
                let slot = fields.get_mut(index).ok_or_else(|| {
                    RtError::new(crate::RtErrorKind::MissingField, "field out of range")
                })?;
                *slot = value;
                Ok(())
            }
            (RtStructFields::Strings(fields), RtValue::String(value)) => {
                let slot = fields.get_mut(index).ok_or_else(|| {
                    RtError::new(crate::RtErrorKind::MissingField, "field out of range")
                })?;
                *slot = value;
                Ok(())
            }
            (fields, value) => {
                let mut values = Self::fields_to_values(fields);
                let slot = values.get_mut(index).ok_or_else(|| {
                    RtError::new(crate::RtErrorKind::MissingField, "field out of range")
                })?;
                *slot = value;
                *fields = RtStructFields::Values(values);
                Ok(())
            }
        }
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

    fn infer_fields(fields: Vec<RtValue>) -> RtStructFields {
        if fields.iter().all(|field| matches!(field, RtValue::Int(_))) {
            return RtStructFields::Ints(
                fields
                    .into_iter()
                    .map(|field| match field {
                        RtValue::Int(value) => value,
                        _ => unreachable!(),
                    })
                    .collect(),
            );
        }
        if fields
            .iter()
            .all(|field| matches!(field, RtValue::Float(_)))
        {
            return RtStructFields::Floats(
                fields
                    .into_iter()
                    .map(|field| match field {
                        RtValue::Float(value) => value,
                        _ => unreachable!(),
                    })
                    .collect(),
            );
        }
        if fields.iter().all(|field| matches!(field, RtValue::Bool(_))) {
            return RtStructFields::Bools(
                fields
                    .into_iter()
                    .map(|field| match field {
                        RtValue::Bool(value) => value,
                        _ => unreachable!(),
                    })
                    .collect(),
            );
        }
        if fields
            .iter()
            .all(|field| matches!(field, RtValue::String(_)))
        {
            return RtStructFields::Strings(
                fields
                    .into_iter()
                    .map(|field| match field {
                        RtValue::String(value) => value,
                        _ => unreachable!(),
                    })
                    .collect(),
            );
        }
        RtStructFields::Values(fields)
    }

    fn fields_to_values(fields: &RtStructFields) -> Vec<RtValue> {
        match fields {
            RtStructFields::Values(fields) => fields.clone(),
            RtStructFields::Ints(fields) => fields.iter().copied().map(RtValue::Int).collect(),
            RtStructFields::Floats(fields) => fields.iter().copied().map(RtValue::Float).collect(),
            RtStructFields::Bools(fields) => fields.iter().copied().map(RtValue::Bool).collect(),
            RtStructFields::Strings(fields) => {
                fields.iter().cloned().map(RtValue::String).collect()
            }
        }
    }
}
