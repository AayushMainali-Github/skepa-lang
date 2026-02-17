use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("random", "seed", builtin_random_seed);
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

