use crate::bytecode::Value;

use super::super::{BuiltinHost, VmError, VmErrorKind};

pub(super) fn builtin_arr_len(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.len expects 1 argument",
        ));
    }
    match &args[0] {
        Value::Array(items) => Ok(Value::Int(items.len() as i64)),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.len expects Array argument",
        )),
    }
}

pub(super) fn builtin_arr_is_empty(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.isEmpty expects 1 argument",
        ));
    }
    match &args[0] {
        Value::Array(items) => Ok(Value::Bool(items.is_empty())),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.isEmpty expects Array argument",
        )),
    }
}

pub(super) fn builtin_arr_contains(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.contains expects 2 arguments",
        ));
    }
    match &args[0] {
        Value::Array(items) => Ok(Value::Bool(items.contains(&args[1]))),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.contains expects Array as first argument",
        )),
    }
}

pub(super) fn builtin_arr_index_of(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.indexOf expects 2 arguments",
        ));
    }
    match &args[0] {
        Value::Array(items) => {
            let idx = items
                .iter()
                .position(|v| v == &args[1])
                .map(|i| i as i64)
                .unwrap_or(-1);
            Ok(Value::Int(idx))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.indexOf expects Array as first argument",
        )),
    }
}

pub(super) fn builtin_arr_count(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.count expects 2 arguments",
        ));
    }
    match &args[0] {
        Value::Array(items) => {
            let c = items.iter().filter(|v| *v == &args[1]).count() as i64;
            Ok(Value::Int(c))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.count expects Array as first argument",
        )),
    }
}

pub(super) fn builtin_arr_first(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.first expects 1 argument",
        ));
    }
    match &args[0] {
        Value::Array(items) => items
            .first()
            .cloned()
            .ok_or_else(|| VmError::new(VmErrorKind::IndexOutOfBounds, "arr.first on empty array")),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.first expects Array argument",
        )),
    }
}

pub(super) fn builtin_arr_last(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.last expects 1 argument",
        ));
    }
    match &args[0] {
        Value::Array(items) => items
            .last()
            .cloned()
            .ok_or_else(|| VmError::new(VmErrorKind::IndexOutOfBounds, "arr.last on empty array")),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.last expects Array argument",
        )),
    }
}
