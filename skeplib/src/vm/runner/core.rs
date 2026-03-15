use crate::bytecode::Value;
use crate::vm::{VmError, VmErrorKind};

use super::{err_at, state};

pub(super) fn load_global(
    globals: &[Value],
    stack: &mut Vec<Value>,
    slot: usize,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let Some(v) = globals.get(slot).cloned() else {
        return Err(err_at(
            VmErrorKind::InvalidLocal,
            format!("Invalid global slot {slot}"),
            function_name,
            ip,
        ));
    };
    stack.push(v);
    Ok(())
}

pub(super) fn store_global(
    globals: &mut Vec<Value>,
    stack: &mut Vec<Value>,
    slot: usize,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let Some(v) = stack.pop() else {
        return Err(err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on StoreGlobal",
            function_name,
            ip,
        ));
    };
    if slot >= globals.len() {
        globals.resize(slot + 1, Value::Unit);
    }
    globals[slot] = v;
    Ok(())
}

pub(super) fn pop(stack: &mut Vec<Value>, function_name: &str, ip: usize) -> Result<(), VmError> {
    if stack.pop().is_none() {
        return Err(err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on Pop",
            function_name,
            ip,
        ));
    }
    Ok(())
}

pub(super) fn finish_return(
    frames: &mut Vec<state::CallFrame<'_>>,
    locals_pool: &mut Vec<Vec<Value>>,
    stack_pool: &mut Vec<Vec<Value>>,
) -> Result<Option<Value>, VmError> {
    let Some(frame) = frames.last_mut() else {
        return Ok(Some(Value::Unit));
    };
    let ret = frame.stack.pop().unwrap_or(Value::Unit);
    if let Some(storage) = frames.pop().map(state::CallFrame::into_storage) {
        locals_pool.push(storage.locals);
        stack_pool.push(storage.stack);
    }
    if let Some(parent) = frames.last_mut() {
        parent.stack.push(ret);
        Ok(None)
    } else {
        Ok(Some(ret))
    }
}
