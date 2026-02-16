use crate::bytecode::Value;

use super::super::{BuiltinHost, VmError, VmErrorKind};

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

pub(super) fn builtin_arr_sum(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
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
