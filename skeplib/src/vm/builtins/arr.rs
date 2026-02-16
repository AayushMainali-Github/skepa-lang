use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("arr", "len", builtin_arr_len);
    r.register("arr", "isEmpty", builtin_arr_is_empty);
    r.register("arr", "contains", builtin_arr_contains);
    r.register("arr", "indexOf", builtin_arr_index_of);
    r.register("arr", "sum", builtin_arr_sum);
    r.register("arr", "count", builtin_arr_count);
    r.register("arr", "first", builtin_arr_first);
    r.register("arr", "last", builtin_arr_last);
    r.register("arr", "reverse", builtin_arr_reverse);
    r.register("arr", "join", builtin_arr_join);
    r.register("arr", "slice", builtin_arr_slice);
    r.register("arr", "min", builtin_arr_min);
    r.register("arr", "max", builtin_arr_max);
    r.register("arr", "sort", builtin_arr_sort);
}

fn builtin_arr_len(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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

fn builtin_arr_is_empty(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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

fn builtin_arr_contains(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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

fn builtin_arr_index_of(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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

fn arr_sum_step(lhs: Value, rhs: Value) -> Result<Value, VmError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),
        (Value::Array(mut a), Value::Array(b)) => {
            a.extend(b);
            Ok(Value::Array(a))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.sum supports Int, Float, String, or Array element types",
        )),
    }
}

fn builtin_arr_sum(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.sum expects 1 argument",
        ));
    }
    let Value::Array(items) = &args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.sum expects Array argument",
        ));
    };
    if items.is_empty() {
        return Ok(Value::Int(0));
    }
    let mut acc = items[0].clone();
    for v in items.iter().skip(1) {
        acc = arr_sum_step(acc, v.clone())?;
    }
    Ok(acc)
}

fn builtin_arr_count(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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

fn builtin_arr_first(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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

fn builtin_arr_last(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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

fn builtin_arr_reverse(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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

fn builtin_arr_join(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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

fn builtin_arr_slice(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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
    Ok(Value::Array(
        items[*start as usize..*end as usize].to_vec(),
    ))
}

fn builtin_arr_min(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.min expects 1 argument",
        ));
    }
    let Value::Array(items) = &args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.min expects Array argument",
        ));
    };
    let first = items
        .first()
        .ok_or_else(|| VmError::new(VmErrorKind::IndexOutOfBounds, "arr.min on empty array"))?;
    match first.clone() {
        Value::Int(mut best) => {
            for v in items.iter().skip(1) {
                let Value::Int(x) = v else {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        "arr.min supports Int or Float element types",
                    ));
                };
                if x < &best {
                    best = *x;
                }
            }
            Ok(Value::Int(best))
        }
        Value::Float(mut best) => {
            for v in items.iter().skip(1) {
                let Value::Float(x) = v else {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        "arr.min supports Int or Float element types",
                    ));
                };
                if x < &best {
                    best = *x;
                }
            }
            Ok(Value::Float(best))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.min supports Int or Float element types",
        )),
    }
}

fn builtin_arr_max(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.max expects 1 argument",
        ));
    }
    let Value::Array(items) = &args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.max expects Array argument",
        ));
    };
    let first = items
        .first()
        .ok_or_else(|| VmError::new(VmErrorKind::IndexOutOfBounds, "arr.max on empty array"))?;
    match first.clone() {
        Value::Int(mut best) => {
            for v in items.iter().skip(1) {
                let Value::Int(x) = v else {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        "arr.max supports Int or Float element types",
                    ));
                };
                if x > &best {
                    best = *x;
                }
            }
            Ok(Value::Int(best))
        }
        Value::Float(mut best) => {
            for v in items.iter().skip(1) {
                let Value::Float(x) = v else {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        "arr.max supports Int or Float element types",
                    ));
                };
                if x > &best {
                    best = *x;
                }
            }
            Ok(Value::Float(best))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.max supports Int or Float element types",
        )),
    }
}

fn builtin_arr_sort(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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
