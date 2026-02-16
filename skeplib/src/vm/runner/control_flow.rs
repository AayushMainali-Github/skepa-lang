use crate::bytecode::Value;
use crate::vm::{VmError, VmErrorKind};

pub(super) fn jump(target: usize) -> usize {
    target
}

pub(super) fn jump_if_false(
    stack: &mut Vec<Value>,
    target: usize,
    function_name: &str,
    ip: usize,
) -> Result<Option<usize>, VmError> {
    let Some(Value::Bool(v)) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "JumpIfFalse expects Bool",
            function_name,
            ip,
        ));
    };
    if !v {
        return Ok(Some(target));
    }
    Ok(None)
}

pub(super) fn jump_if_true(
    stack: &mut Vec<Value>,
    target: usize,
    function_name: &str,
    ip: usize,
) -> Result<Option<usize>, VmError> {
    let Some(Value::Bool(v)) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "JumpIfTrue expects Bool",
            function_name,
            ip,
        ));
    };
    if v {
        return Ok(Some(target));
    }
    Ok(None)
}
