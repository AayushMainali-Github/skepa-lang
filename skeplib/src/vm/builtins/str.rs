use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

const MAX_STR_REPEAT_OUTPUT_BYTES: usize = 1_000_000;

fn scalar_len(s: &str) -> i64 {
    if s.is_ascii() {
        s.len() as i64
    } else {
        s.chars().count() as i64
    }
}

fn scalar_prefix_len(s: &str, byte_idx: usize) -> i64 {
    if s.is_ascii() {
        byte_idx as i64
    } else {
        s[..byte_idx].chars().count() as i64
    }
}

#[allow(dead_code)]
pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("str", "len", builtin_str_len);
    r.register("str", "contains", builtin_str_contains);
    r.register("str", "startsWith", builtin_str_starts_with);
    r.register("str", "endsWith", builtin_str_ends_with);
    r.register("str", "trim", builtin_str_trim);
    r.register("str", "toLower", builtin_str_to_lower);
    r.register("str", "toUpper", builtin_str_to_upper);
    r.register("str", "indexOf", builtin_str_index_of);
    r.register("str", "slice", builtin_str_slice);
    r.register("str", "isEmpty", builtin_str_is_empty);
    r.register("str", "lastIndexOf", builtin_str_last_index_of);
    r.register("str", "replace", builtin_str_replace);
    r.register("str", "repeat", builtin_str_repeat);
}

pub(crate) fn builtin_str_len(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.len expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Int(scalar_len(s))),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.len expects String argument",
        )),
    }
}

pub(crate) fn builtin_str_contains(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.contains expects 2 arguments",
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(h), Value::String(n)) => Ok(Value::Bool(h.contains(n.as_ref()))),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.contains expects String, String arguments",
        )),
    }
}

pub(crate) fn builtin_str_starts_with(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.startsWith expects 2 arguments",
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(p)) => Ok(Value::Bool(s.starts_with(p.as_ref()))),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.startsWith expects String, String arguments",
        )),
    }
}

pub(crate) fn builtin_str_ends_with(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.endsWith expects 2 arguments",
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(p)) => Ok(Value::Bool(s.ends_with(p.as_ref()))),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.endsWith expects String, String arguments",
        )),
    }
}

pub(crate) fn builtin_str_trim(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.trim expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.trim().to_string().into())),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.trim expects String argument",
        )),
    }
}

pub(crate) fn builtin_str_to_lower(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.toLower expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_lowercase().into())),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.toLower expects String argument",
        )),
    }
}

pub(crate) fn builtin_str_to_upper(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.toUpper expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_uppercase().into())),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.toUpper expects String argument",
        )),
    }
}

pub(crate) fn builtin_str_index_of(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.indexOf expects 2 arguments",
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(n)) => match s.find(n.as_ref()) {
            Some(byte_idx) => Ok(Value::Int(scalar_prefix_len(s, byte_idx))),
            None => Ok(Value::Int(-1)),
        },
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.indexOf expects String, String arguments",
        )),
    }
}

pub(crate) fn builtin_str_slice(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 3 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.slice expects 3 arguments",
        ));
    }
    let (Value::String(s), Value::Int(start), Value::Int(end)) = (&args[0], &args[1], &args[2])
    else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.slice expects String, Int, Int arguments",
        ));
    };
    let len = scalar_len(s);
    if *start < 0 || *end < 0 || *start > *end || *end > len {
        return Err(VmError::new(
            VmErrorKind::IndexOutOfBounds,
            format!(
                "str.slice bounds out of range: start={}, end={}, len={len}",
                start, end
            ),
        ));
    }
    let out = if s.is_ascii() {
        s[*start as usize..*end as usize].to_string()
    } else {
        s.chars()
            .skip(*start as usize)
            .take((*end - *start) as usize)
            .collect()
    };
    Ok(Value::String(out.into()))
}

pub(crate) fn builtin_str_is_empty(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.isEmpty expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Bool(s.is_empty())),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.isEmpty expects String argument",
        )),
    }
}

pub(crate) fn builtin_str_last_index_of(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.lastIndexOf expects 2 arguments",
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(n)) => match s.rfind(n.as_ref()) {
            Some(byte_idx) => Ok(Value::Int(scalar_prefix_len(s, byte_idx))),
            None => Ok(Value::Int(-1)),
        },
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.lastIndexOf expects String, String arguments",
        )),
    }
}

pub(crate) fn builtin_str_replace(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 3 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.replace expects 3 arguments",
        ));
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::String(s), Value::String(from), Value::String(to)) => {
            Ok(Value::String(s.replace(from.as_ref(), to.as_ref()).into()))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.replace expects String, String, String arguments",
        )),
    }
}

pub(crate) fn builtin_str_repeat(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.repeat expects 2 arguments",
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::Int(n)) => {
            if *n < 0 {
                return Err(VmError::new(
                    VmErrorKind::IndexOutOfBounds,
                    "str.repeat count must be >= 0",
                ));
            }
            let count = *n as usize;
            let out_len = s.len().checked_mul(count).ok_or_else(|| {
                VmError::new(VmErrorKind::IndexOutOfBounds, "str.repeat output too large")
            })?;
            if out_len > MAX_STR_REPEAT_OUTPUT_BYTES {
                return Err(VmError::new(
                    VmErrorKind::IndexOutOfBounds,
                    format!(
                        "str.repeat output too large: {} bytes exceeds limit {}",
                        out_len, MAX_STR_REPEAT_OUTPUT_BYTES
                    ),
                ));
            }
            Ok(Value::String(s.repeat(count).into()))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.repeat expects String, Int arguments",
        )),
    }
}
