use crate::bytecode::Value;

use super::super::{BuiltinHost, VmError, VmErrorKind};

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
