use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

#[allow(dead_code)]
pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("vec", "new", builtin_vec_new);
    r.register("vec", "len", builtin_vec_len);
    r.register("vec", "push", builtin_vec_push);
    r.register("vec", "get", builtin_vec_get);
    r.register("vec", "set", builtin_vec_set);
    r.register("vec", "delete", builtin_vec_delete);
}

fn expect_vec_handle(arg: &Value, fn_name: &str, arg_pos: usize) -> Result<u64, VmError> {
    match arg {
        Value::VecHandle(id) => Ok(*id),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            format!("vec.{fn_name} argument {arg_pos} expects Vec"),
        )),
    }
}

fn expect_int(arg: &Value, fn_name: &str, arg_pos: usize) -> Result<i64, VmError> {
    match arg {
        Value::Int(i) => Ok(*i),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            format!("vec.{fn_name} argument {arg_pos} expects Int"),
        )),
    }
}

pub(crate) fn builtin_vec_new(
    host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if !args.is_empty() {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "vec.new expects 0 arguments",
        ));
    }
    Ok(Value::VecHandle(host.vec_new()?))
}

pub(crate) fn builtin_vec_len(
    host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "vec.len expects 1 argument",
        ));
    }
    let id = expect_vec_handle(&args[0], "len", 1)?;
    Ok(Value::Int(host.vec_len(id)? as i64))
}

pub(crate) fn builtin_vec_push(
    host: &mut dyn BuiltinHost,
    mut args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "vec.push expects 2 arguments",
        ));
    }
    let value = args.pop().expect("len checked");
    let handle = args.pop().expect("len checked");
    let id = expect_vec_handle(&handle, "push", 1)?;
    host.vec_push(id, value)?;
    Ok(Value::Unit)
}

pub(crate) fn builtin_vec_get(
    host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "vec.get expects 2 arguments",
        ));
    }
    let id = expect_vec_handle(&args[0], "get", 1)?;
    let idx = expect_int(&args[1], "get", 2)?;
    host.vec_get(id, idx)
}

pub(crate) fn builtin_vec_set(
    host: &mut dyn BuiltinHost,
    mut args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 3 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "vec.set expects 3 arguments",
        ));
    }
    let value = args.pop().expect("len checked");
    let idx_val = args.pop().expect("len checked");
    let handle = args.pop().expect("len checked");
    let id = expect_vec_handle(&handle, "set", 1)?;
    let idx = expect_int(&idx_val, "set", 2)?;
    host.vec_set(id, idx, value)?;
    Ok(Value::Unit)
}

pub(crate) fn builtin_vec_delete(
    host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "vec.delete expects 2 arguments",
        ));
    }
    let id = expect_vec_handle(&args[0], "delete", 1)?;
    let idx = expect_int(&args[1], "delete", 2)?;
    host.vec_delete(id, idx)
}
