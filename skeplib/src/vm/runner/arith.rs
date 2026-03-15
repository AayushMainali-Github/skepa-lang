use crate::bytecode::{Instr, IntLocalConstOp, Value};
use crate::vm::{VmError, VmErrorKind};

use super::{err_at, invalid_local_slot, state};

pub(super) fn neg(stack: &mut Vec<Value>, function_name: &str, ip: usize) -> Result<(), VmError> {
    let Some(v) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "NegInt expects value",
            function_name,
            ip,
        ));
    };
    match v {
        Value::Int(v) => stack.push(Value::Int(-v)),
        Value::Float(v) => stack.push(Value::Float(-v)),
        _ => {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                "NegInt expects Int or Float",
                function_name,
                ip,
            ));
        }
    }
    Ok(())
}

pub(super) fn not_bool(
    stack: &mut Vec<Value>,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let Some(Value::Bool(v)) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "NotBool expects Bool",
            function_name,
            ip,
        ));
    };
    stack.push(Value::Bool(!v));
    Ok(())
}

pub(super) fn add(stack: &mut Vec<Value>, function_name: &str, ip: usize) -> Result<(), VmError> {
    let Some(r) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Add expects rhs",
            function_name,
            ip,
        ));
    };
    let Some(l) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Add expects lhs",
            function_name,
            ip,
        ));
    };
    match (l, r) {
        (Value::Int(a), Value::Int(b)) => stack.push(Value::Int(a + b)),
        (Value::Float(a), Value::Float(b)) => stack.push(Value::Float(a + b)),
        (Value::String(a), Value::String(b)) => stack.push(Value::String(format!("{a}{b}").into())),
        (Value::Array(a), Value::Array(b)) => {
            let mut joined = a.as_ref().to_vec();
            joined.extend(b.iter().cloned());
            stack.push(Value::Array(joined.into()));
        }
        _ => {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                "Add supports Int+Int, Float+Float, String+String, or Array+Array",
                function_name,
                ip,
            ));
        }
    }
    Ok(())
}

pub(super) fn numeric_binop(
    stack: &mut Vec<Value>,
    instr: &Instr,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let Some(r) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "int binary op expects rhs",
            function_name,
            ip,
        ));
    };
    let Some(l) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "int binary op expects lhs",
            function_name,
            ip,
        ));
    };
    match (l, r) {
        (Value::Int(l), Value::Int(r)) => match instr {
            Instr::SubInt => stack.push(Value::Int(l - r)),
            Instr::MulInt => stack.push(Value::Int(l * r)),
            Instr::DivInt => {
                if r == 0 {
                    return Err(super::err_at(
                        VmErrorKind::DivisionByZero,
                        "division by zero",
                        function_name,
                        ip,
                    ));
                }
                stack.push(Value::Int(l / r));
            }
            Instr::LtInt => stack.push(Value::Bool(l < r)),
            Instr::LteInt => stack.push(Value::Bool(l <= r)),
            Instr::GtInt => stack.push(Value::Bool(l > r)),
            Instr::GteInt => stack.push(Value::Bool(l >= r)),
            _ => unreachable!(),
        },
        (Value::Float(l), Value::Float(r)) => match instr {
            Instr::SubInt => stack.push(Value::Float(l - r)),
            Instr::MulInt => stack.push(Value::Float(l * r)),
            Instr::DivInt => {
                if r == 0.0 {
                    return Err(super::err_at(
                        VmErrorKind::DivisionByZero,
                        "division by zero",
                        function_name,
                        ip,
                    ));
                }
                stack.push(Value::Float(l / r));
            }
            Instr::LtInt => stack.push(Value::Bool(l < r)),
            Instr::LteInt => stack.push(Value::Bool(l <= r)),
            Instr::GtInt => stack.push(Value::Bool(l > r)),
            Instr::GteInt => stack.push(Value::Bool(l >= r)),
            _ => unreachable!(),
        },
        _ => {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                "numeric binary op expects matching Int/Float operands",
                function_name,
                ip,
            ));
        }
    }
    Ok(())
}

pub(super) fn mod_int(
    stack: &mut Vec<Value>,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let Some(Value::Int(r)) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "ModInt expects rhs Int",
            function_name,
            ip,
        ));
    };
    let Some(Value::Int(l)) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "ModInt expects lhs Int",
            function_name,
            ip,
        ));
    };
    if r == 0 {
        return Err(super::err_at(
            VmErrorKind::DivisionByZero,
            "modulo by zero",
            function_name,
            ip,
        ));
    }
    stack.push(Value::Int(l % r));
    Ok(())
}

pub(super) fn eq(stack: &mut Vec<Value>, function_name: &str, ip: usize) -> Result<(), VmError> {
    let Some(r) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Eq expects rhs",
            function_name,
            ip,
        ));
    };
    let Some(l) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Eq expects lhs",
            function_name,
            ip,
        ));
    };
    if matches!(l, Value::Function(_) | Value::FunctionIdx(_))
        || matches!(r, Value::Function(_) | Value::FunctionIdx(_))
    {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "Eq does not support Function values",
            function_name,
            ip,
        ));
    }
    stack.push(Value::Bool(l == r));
    Ok(())
}

pub(super) fn neq(stack: &mut Vec<Value>, function_name: &str, ip: usize) -> Result<(), VmError> {
    let Some(r) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Neq expects rhs",
            function_name,
            ip,
        ));
    };
    let Some(l) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Neq expects lhs",
            function_name,
            ip,
        ));
    };
    if matches!(l, Value::Function(_) | Value::FunctionIdx(_))
        || matches!(r, Value::Function(_) | Value::FunctionIdx(_))
    {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "Neq does not support Function values",
            function_name,
            ip,
        ));
    }
    stack.push(Value::Bool(l != r));
    Ok(())
}

pub(super) fn logical(
    stack: &mut Vec<Value>,
    instr: &Instr,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    let Some(Value::Bool(r)) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "logical op expects rhs Bool",
            function_name,
            ip,
        ));
    };
    let Some(Value::Bool(l)) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "logical op expects lhs Bool",
            function_name,
            ip,
        ));
    };
    match instr {
        Instr::AndBool => stack.push(Value::Bool(l && r)),
        Instr::OrBool => stack.push(Value::Bool(l || r)),
        _ => unreachable!(),
    }
    Ok(())
}

pub(super) fn handle_instr(
    frame: &mut state::CallFrame<'_>,
    instr: &Instr,
    function_name: &str,
    ip: usize,
) -> Result<bool, VmError> {
    match instr {
        Instr::NegInt => {
            neg(&mut frame.stack, function_name, ip)?;
            Ok(true)
        }
        Instr::NotBool => {
            not_bool(&mut frame.stack, function_name, ip)?;
            Ok(true)
        }
        Instr::IntLocalLocalOp { lhs, rhs, op } => {
            let Some(left) = frame.locals.get(*lhs).cloned() else {
                return Err(invalid_local_slot(function_name, ip, *lhs));
            };
            let Some(right) = frame.locals.get(*rhs).cloned() else {
                return Err(invalid_local_slot(function_name, ip, *rhs));
            };
            match (left, right) {
                (Value::Int(lhs), Value::Int(rhs)) => {
                    let result = match op {
                        IntLocalConstOp::Add => Value::Int(lhs + rhs),
                        IntLocalConstOp::Sub => Value::Int(lhs - rhs),
                        IntLocalConstOp::Mul => Value::Int(lhs * rhs),
                        IntLocalConstOp::Div => {
                            if rhs == 0 {
                                return Err(err_at(
                                    VmErrorKind::DivisionByZero,
                                    "division by zero",
                                    function_name,
                                    ip,
                                ));
                            }
                            Value::Int(lhs / rhs)
                        }
                        IntLocalConstOp::Mod => {
                            if rhs == 0 {
                                return Err(err_at(
                                    VmErrorKind::DivisionByZero,
                                    "modulo by zero",
                                    function_name,
                                    ip,
                                ));
                            }
                            Value::Int(lhs % rhs)
                        }
                    };
                    frame.stack.push(result);
                }
                (left, right) => {
                    frame.stack.push(left);
                    frame.stack.push(right);
                    match op {
                        IntLocalConstOp::Add => add(&mut frame.stack, function_name, ip)?,
                        IntLocalConstOp::Sub | IntLocalConstOp::Mul | IntLocalConstOp::Div => {
                            let generic_instr = match op {
                                IntLocalConstOp::Sub => &Instr::SubInt,
                                IntLocalConstOp::Mul => &Instr::MulInt,
                                IntLocalConstOp::Div => &Instr::DivInt,
                                IntLocalConstOp::Add | IntLocalConstOp::Mod => unreachable!(),
                            };
                            numeric_binop(&mut frame.stack, generic_instr, function_name, ip)?
                        }
                        IntLocalConstOp::Mod => mod_int(&mut frame.stack, function_name, ip)?,
                    }
                }
            }
            Ok(true)
        }
        Instr::SubInt | Instr::MulInt | Instr::DivInt | Instr::GtInt | Instr::GteInt => {
            numeric_binop(&mut frame.stack, instr, function_name, ip)?;
            Ok(true)
        }
        Instr::ModInt => {
            let stack = &mut frame.stack;
            let Some(r) = stack.pop() else {
                return Err(err_at(
                    VmErrorKind::TypeMismatch,
                    "ModInt expects rhs Int",
                    function_name,
                    ip,
                ));
            };
            let Some(l) = stack.pop() else {
                stack.push(r);
                return Err(err_at(
                    VmErrorKind::TypeMismatch,
                    "ModInt expects lhs Int",
                    function_name,
                    ip,
                ));
            };
            match (l, r) {
                (Value::Int(l), Value::Int(r)) => {
                    if r == 0 {
                        return Err(err_at(
                            VmErrorKind::DivisionByZero,
                            "modulo by zero",
                            function_name,
                            ip,
                        ));
                    }
                    stack.push(Value::Int(l % r));
                }
                (l, r) => {
                    stack.push(l);
                    stack.push(r);
                    mod_int(stack, function_name, ip)?
                }
            }
            Ok(true)
        }
        Instr::Eq => {
            eq(&mut frame.stack, function_name, ip)?;
            Ok(true)
        }
        Instr::Neq => {
            neq(&mut frame.stack, function_name, ip)?;
            Ok(true)
        }
        Instr::AndBool | Instr::OrBool => {
            logical(&mut frame.stack, instr, function_name, ip)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
