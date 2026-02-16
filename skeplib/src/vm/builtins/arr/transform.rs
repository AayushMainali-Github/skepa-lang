use crate::bytecode::Value;

use super::super::{BuiltinHost, VmError, VmErrorKind};

pub(super) fn builtin_arr_reverse(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.reverse expects 1 argument",
        ));
    }
    match &args[0] {
        Value::Array(items) => {
            let mut out = items.clone();
            out.reverse();
            Ok(Value::Array(out))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.reverse expects Array argument",
        )),
    }
}

pub(super) fn builtin_arr_join(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.join expects 2 arguments",
        ));
    }
    let Value::Array(items) = &args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.join expects Array as first argument",
        ));
    };
    let Value::String(sep) = &args[1] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.join expects String as second argument",
        ));
    };
    let mut parts = Vec::with_capacity(items.len());
    for item in items {
        let Value::String(s) = item else {
            return Err(VmError::new(
                VmErrorKind::TypeMismatch,
                "arr.join expects Array[String] as first argument",
            ));
        };
        parts.push(s.clone());
    }
    Ok(Value::String(parts.join(sep)))
}

pub(super) fn builtin_arr_slice(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 3 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.slice expects 3 arguments",
        ));
    }
    let (Value::Array(items), Value::Int(start), Value::Int(end)) = (&args[0], &args[1], &args[2])
    else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.slice expects Array, Int, Int arguments",
        ));
    };
    let len = items.len() as i64;
    if *start < 0 || *end < 0 || *start > *end || *end > len {
        return Err(VmError::new(
            VmErrorKind::IndexOutOfBounds,
            format!(
                "arr.slice bounds out of range: start={}, end={}, len={len}",
                start, end
            ),
        ));
    }
    Ok(Value::Array(items[*start as usize..*end as usize].to_vec()))
}

pub(super) fn builtin_arr_sort(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.sort expects 1 argument",
        ));
    }
    let Value::Array(items) = &args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.sort expects Array argument",
        ));
    };
    let out = items.clone();
    match out.first() {
        None => Ok(Value::Array(out)),
        Some(Value::Int(_)) => {
            let mut nums = Vec::with_capacity(out.len());
            for v in &out {
                let Value::Int(x) = v else {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        "arr.sort supports Int, Float, String, or Bool element types",
                    ));
                };
                nums.push(*x);
            }
            nums.sort_unstable();
            Ok(Value::Array(nums.into_iter().map(Value::Int).collect()))
        }
        Some(Value::Float(_)) => {
            let mut nums = Vec::with_capacity(out.len());
            for v in &out {
                let Value::Float(x) = v else {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        "arr.sort supports Int, Float, String, or Bool element types",
                    ));
                };
                nums.push(*x);
            }
            nums.sort_by(|a, b| a.total_cmp(b));
            Ok(Value::Array(nums.into_iter().map(Value::Float).collect()))
        }
        Some(Value::String(_)) => {
            let mut strs = Vec::with_capacity(out.len());
            for v in &out {
                let Value::String(x) = v else {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        "arr.sort supports Int, Float, String, or Bool element types",
                    ));
                };
                strs.push(x.clone());
            }
            strs.sort();
            Ok(Value::Array(strs.into_iter().map(Value::String).collect()))
        }
        Some(Value::Bool(_)) => {
            let mut bs = Vec::with_capacity(out.len());
            for v in &out {
                let Value::Bool(x) = v else {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        "arr.sort supports Int, Float, String, or Bool element types",
                    ));
                };
                bs.push(*x);
            }
            bs.sort();
            Ok(Value::Array(bs.into_iter().map(Value::Bool).collect()))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.sort supports Int, Float, String, or Bool element types",
        )),
    }
}
