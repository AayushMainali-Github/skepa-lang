use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

#[allow(dead_code)]
pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("random", "seed", builtin_random_seed);
    r.register("random", "int", builtin_random_int);
    r.register("random", "float", builtin_random_float);
}

pub(crate) fn builtin_random_seed(
    host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "random.seed expects 1 argument",
        ));
    }
    let Value::Int(seed) = args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "random.seed expects Int argument",
        ));
    };
    host.set_random_seed(seed as u64)?;
    Ok(Value::Unit)
}

pub(crate) fn builtin_random_int(
    host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "random.int expects 2 arguments",
        ));
    }
    let Value::Int(min) = args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "random.int argument 1 expects Int",
        ));
    };
    let Value::Int(max) = args[1] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "random.int argument 2 expects Int",
        ));
    };
    if min > max {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "random.int expects min <= max",
        ));
    }
    let span = (max - min) as u64 + 1;
    let r = host.next_random_u64()?;
    let offset = (r % span) as i64;
    Ok(Value::Int(min + offset))
}

pub(crate) fn builtin_random_float(
    host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if !args.is_empty() {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "random.float expects 0 arguments",
        ));
    }
    let r = host.next_random_u64()?;
    let f = (r as f64) / ((u64::MAX as f64) + 1.0);
    Ok(Value::Float(f))
}
