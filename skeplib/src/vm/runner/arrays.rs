use crate::bytecode::Value;
use crate::vm::{VmError, VmErrorKind};
use std::rc::Rc;

fn string_scalar_len(s: &str) -> i64 {
    if s.is_ascii() {
        s.len() as i64
    } else {
        s.chars().count() as i64
    }
}

pub(super) fn make_array(
    stack: &mut Vec<Value>,
    n: usize,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let _timer = super::super::profiler::ScopedTimer::new(super::super::profiler::Event::ArrayMake);
    if stack.len() < n {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "MakeArray expects enough stack values",
            function_name,
            ip,
        ));
    }
    let start = stack.len() - n;
    let items = stack.split_off(start);
    stack.push(Value::Array(Rc::<[Value]>::from(items)));
    Ok(())
}

pub(super) fn make_array_repeat(
    stack: &mut Vec<Value>,
    n: usize,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let _timer =
        super::super::profiler::ScopedTimer::new(super::super::profiler::Event::ArrayMakeRepeat);
    let Some(v) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "MakeArrayRepeat expects a value",
            function_name,
            ip,
        ));
    };
    stack.push(Value::Array(Rc::<[Value]>::from(vec![v; n])));
    Ok(())
}

pub(super) fn array_get(
    stack: &mut Vec<Value>,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let _timer = super::super::profiler::ScopedTimer::new(super::super::profiler::Event::ArrayGet);
    let Some(idx_v) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "ArrayGet expects index",
            function_name,
            ip,
        ));
    };
    let Some(arr_v) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "ArrayGet expects array",
            function_name,
            ip,
        ));
    };
    let Value::Int(idx) = idx_v else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "ArrayGet index must be Int",
            function_name,
            ip,
        ));
    };
    let Value::Array(items) = arr_v else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "ArrayGet expects Array",
            function_name,
            ip,
        ));
    };
    if idx < 0 || idx as usize >= items.len() {
        return Err(super::err_at(
            VmErrorKind::IndexOutOfBounds,
            format!("Array index {} out of bounds (len={})", idx, items.len()),
            function_name,
            ip,
        ));
    }
    stack.push(items[idx as usize].clone());
    Ok(())
}

pub(super) fn array_set(
    stack: &mut Vec<Value>,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let _timer = super::super::profiler::ScopedTimer::new(super::super::profiler::Event::ArraySet);
    let Some(val) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "ArraySet expects value",
            function_name,
            ip,
        ));
    };
    let Some(idx_v) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "ArraySet expects index",
            function_name,
            ip,
        ));
    };
    let Some(arr_v) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "ArraySet expects array",
            function_name,
            ip,
        ));
    };
    let Value::Int(idx) = idx_v else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "ArraySet index must be Int",
            function_name,
            ip,
        ));
    };
    let Value::Array(items) = arr_v else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "ArraySet expects Array",
            function_name,
            ip,
        ));
    };
    let mut items = items.as_ref().to_vec();
    if idx < 0 || idx as usize >= items.len() {
        return Err(super::err_at(
            VmErrorKind::IndexOutOfBounds,
            format!("Array index {} out of bounds (len={})", idx, items.len()),
            function_name,
            ip,
        ));
    }
    items[idx as usize] = val;
    stack.push(Value::Array(Rc::<[Value]>::from(items)));
    Ok(())
}

pub(super) fn array_set_chain(
    stack: &mut Vec<Value>,
    depth: usize,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let _timer =
        super::super::profiler::ScopedTimer::new(super::super::profiler::Event::ArraySetChain);
    if depth == 0 {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "ArraySetChain depth must be > 0",
            function_name,
            ip,
        ));
    }
    let Some(val) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "ArraySetChain expects value",
            function_name,
            ip,
        ));
    };
    if stack.len() < depth + 1 {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "ArraySetChain expects array and all indices",
            function_name,
            ip,
        ));
    }
    let mut indices = Vec::with_capacity(depth);
    for _ in 0..depth {
        let Some(idx_v) = stack.pop() else {
            return Err(super::err_at(
                VmErrorKind::StackUnderflow,
                "ArraySetChain expects index",
                function_name,
                ip,
            ));
        };
        let Value::Int(idx) = idx_v else {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                "ArraySetChain index must be Int",
                function_name,
                ip,
            ));
        };
        indices.push(idx);
    }
    indices.reverse();
    let Some(arr_v) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "ArraySetChain expects array",
            function_name,
            ip,
        ));
    };

    match set_deep(arr_v, &indices, val) {
        Ok(updated) => stack.push(updated),
        Err(VmErrorKind::TypeMismatch) => {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                "ArraySetChain expects nested arrays along the assignment path",
                function_name,
                ip,
            ));
        }
        Err(VmErrorKind::IndexOutOfBounds) => {
            return Err(super::err_at(
                VmErrorKind::IndexOutOfBounds,
                "ArraySetChain index out of bounds",
                function_name,
                ip,
            ));
        }
        Err(other) => {
            return Err(super::err_at(
                other,
                "ArraySetChain failed",
                function_name,
                ip,
            ));
        }
    }
    Ok(())
}

pub(super) fn array_len(
    stack: &mut Vec<Value>,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let _timer = super::super::profiler::ScopedTimer::new(super::super::profiler::Event::ArrayLen);
    let Some(arr_v) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "ArrayLen expects value",
            function_name,
            ip,
        ));
    };
    match arr_v {
        Value::Array(items) => stack.push(Value::Int(items.len() as i64)),
        Value::String(s) => stack.push(Value::Int(string_scalar_len(&s))),
        _ => {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                "len expects Array or String",
                function_name,
                ip,
            ));
        }
    }
    Ok(())
}

fn set_deep(cur: Value, indices: &[i64], val: Value) -> Result<Value, VmErrorKind> {
    let Value::Array(items) = cur else {
        return Err(VmErrorKind::TypeMismatch);
    };
    let mut items = items.as_ref().to_vec();
    let idx = indices[0];
    if idx < 0 || idx as usize >= items.len() {
        return Err(VmErrorKind::IndexOutOfBounds);
    }
    let u = idx as usize;
    if indices.len() == 1 {
        items[u] = val;
        return Ok(Value::Array(Rc::<[Value]>::from(items)));
    }
    let child = items[u].clone();
    let next = set_deep(child, &indices[1..], val)?;
    items[u] = next;
    Ok(Value::Array(Rc::<[Value]>::from(items)))
}
