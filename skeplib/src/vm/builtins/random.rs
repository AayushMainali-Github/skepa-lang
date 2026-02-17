use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("random", "seed", builtin_random_seed);
    r.register("random", "int", builtin_random_int);
    r.register("random", "float", builtin_random_float);
}

fn builtin_random_seed(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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

fn builtin_random_int(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if !args.is_empty() {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "random.int expects 0 arguments",
        ));
    }
    let r = host.next_random_u64()?;
    Ok(Value::Int((r >> 1) as i64))
}

fn builtin_random_float(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
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
