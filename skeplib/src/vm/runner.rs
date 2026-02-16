mod arrays;

use crate::bytecode::{BytecodeModule, Instr, Value};

use super::{BuiltinHost, BuiltinRegistry, VmConfig, VmError, VmErrorKind};

pub(super) fn run_function(
    module: &BytecodeModule,
    function_name: &str,
    args: Vec<Value>,
    host: &mut dyn BuiltinHost,
    reg: &BuiltinRegistry,
    depth: usize,
    config: VmConfig,
) -> Result<Value, VmError> {
    if depth >= config.max_call_depth {
        return Err(VmError::new(
            VmErrorKind::StackOverflow,
            format!("Call stack limit exceeded ({})", config.max_call_depth),
        ));
    }
    let Some(chunk) = module.functions.get(function_name) else {
        return Err(VmError::new(
            VmErrorKind::UnknownFunction,
            format!("Unknown function `{function_name}`"),
        ));
    };
    if args.len() != chunk.param_count {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            format!(
                "Function `{}` arity mismatch: expected {}, got {}",
                function_name,
                chunk.param_count,
                args.len()
            ),
        ));
    }

    let mut stack: Vec<Value> = Vec::new();
    let mut locals: Vec<Value> = vec![Value::Unit; chunk.locals_count.max(1)];
    for (i, arg) in args.into_iter().enumerate() {
        if i < locals.len() {
            locals[i] = arg;
        }
    }

    let mut ip = 0usize;
    while ip < chunk.code.len() {
        if config.trace {
            eprintln!("[trace] {}@{} {:?}", function_name, ip, chunk.code[ip]);
        }
        match &chunk.code[ip] {
            Instr::LoadConst(v) => stack.push(v.clone()),
            Instr::LoadLocal(slot) => {
                let Some(v) = locals.get(*slot).cloned() else {
                    return Err(err_at(
                        VmErrorKind::InvalidLocal,
                        format!("Invalid local slot {slot}"),
                        function_name,
                        ip,
                    ));
                };
                stack.push(v);
            }
            Instr::StoreLocal(slot) => {
                let Some(v) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Stack underflow on StoreLocal",
                        function_name,
                        ip,
                    ));
                };
                if *slot >= locals.len() {
                    locals.resize(*slot + 1, Value::Unit);
                }
                locals[*slot] = v;
            }
            Instr::Pop => {
                if stack.pop().is_none() {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Stack underflow on Pop",
                        function_name,
                        ip,
                    ));
                }
            }
            Instr::NegInt => {
                let Some(v) = stack.pop() else {
                    return Err(err_at(
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
                        return Err(err_at(
                            VmErrorKind::TypeMismatch,
                            "NegInt expects Int or Float",
                            function_name,
                            ip,
                        ));
                    }
                }
            }
            Instr::NotBool => {
                let Some(Value::Bool(v)) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::TypeMismatch,
                        "NotBool expects Bool",
                        function_name,
                        ip,
                    ));
                };
                stack.push(Value::Bool(!v));
            }
            Instr::Add => {
                let Some(r) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Add expects rhs",
                        function_name,
                        ip,
                    ));
                };
                let Some(l) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Add expects lhs",
                        function_name,
                        ip,
                    ));
                };
                match (l, r) {
                    (Value::Int(a), Value::Int(b)) => stack.push(Value::Int(a + b)),
                    (Value::Float(a), Value::Float(b)) => stack.push(Value::Float(a + b)),
                    (Value::String(a), Value::String(b)) => {
                        stack.push(Value::String(format!("{a}{b}")))
                    }
                    (Value::Array(mut a), Value::Array(b)) => {
                        a.extend(b);
                        stack.push(Value::Array(a));
                    }
                    _ => {
                        return Err(err_at(
                            VmErrorKind::TypeMismatch,
                            "Add supports Int+Int, Float+Float, String+String, or Array+Array",
                            function_name,
                            ip,
                        ));
                    }
                }
            }
            Instr::SubInt
            | Instr::MulInt
            | Instr::DivInt
            | Instr::LtInt
            | Instr::LteInt
            | Instr::GtInt
            | Instr::GteInt => {
                let Some(r) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "int binary op expects rhs",
                        function_name,
                        ip,
                    ));
                };
                let Some(l) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "int binary op expects lhs",
                        function_name,
                        ip,
                    ));
                };
                match (l, r) {
                    (Value::Int(l), Value::Int(r)) => match chunk.code[ip] {
                        Instr::SubInt => stack.push(Value::Int(l - r)),
                        Instr::MulInt => stack.push(Value::Int(l * r)),
                        Instr::DivInt => {
                            if r == 0 {
                                return Err(err_at(
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
                    (Value::Float(l), Value::Float(r)) => match chunk.code[ip] {
                        Instr::SubInt => stack.push(Value::Float(l - r)),
                        Instr::MulInt => stack.push(Value::Float(l * r)),
                        Instr::DivInt => {
                            if r == 0.0 {
                                return Err(err_at(
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
                        return Err(err_at(
                            VmErrorKind::TypeMismatch,
                            "numeric binary op expects matching Int/Float operands",
                            function_name,
                            ip,
                        ));
                    }
                }
            }
            Instr::ModInt => {
                let Some(Value::Int(r)) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::TypeMismatch,
                        "ModInt expects rhs Int",
                        function_name,
                        ip,
                    ));
                };
                let Some(Value::Int(l)) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::TypeMismatch,
                        "ModInt expects lhs Int",
                        function_name,
                        ip,
                    ));
                };
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
            Instr::Eq => {
                let Some(r) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Eq expects rhs",
                        function_name,
                        ip,
                    ));
                };
                let Some(l) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Eq expects lhs",
                        function_name,
                        ip,
                    ));
                };
                stack.push(Value::Bool(l == r));
            }
            Instr::Neq => {
                let Some(r) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Neq expects rhs",
                        function_name,
                        ip,
                    ));
                };
                let Some(l) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Neq expects lhs",
                        function_name,
                        ip,
                    ));
                };
                stack.push(Value::Bool(l != r));
            }
            Instr::AndBool | Instr::OrBool => {
                let Some(Value::Bool(r)) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::TypeMismatch,
                        "logical op expects rhs Bool",
                        function_name,
                        ip,
                    ));
                };
                let Some(Value::Bool(l)) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::TypeMismatch,
                        "logical op expects lhs Bool",
                        function_name,
                        ip,
                    ));
                };
                match chunk.code[ip] {
                    Instr::AndBool => stack.push(Value::Bool(l && r)),
                    Instr::OrBool => stack.push(Value::Bool(l || r)),
                    _ => unreachable!(),
                }
            }
            Instr::Jump(target) => {
                ip = *target;
                continue;
            }
            Instr::JumpIfFalse(target) => {
                let Some(Value::Bool(v)) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::TypeMismatch,
                        "JumpIfFalse expects Bool",
                        function_name,
                        ip,
                    ));
                };
                if !v {
                    ip = *target;
                    continue;
                }
            }
            Instr::JumpIfTrue(target) => {
                let Some(Value::Bool(v)) = stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::TypeMismatch,
                        "JumpIfTrue expects Bool",
                        function_name,
                        ip,
                    ));
                };
                if v {
                    ip = *target;
                    continue;
                }
            }
            Instr::Call {
                name: callee_name,
                argc,
            } => {
                if stack.len() < *argc {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Stack underflow on Call",
                        function_name,
                        ip,
                    ));
                }
                let split = stack.len() - *argc;
                let call_args = stack.split_off(split);
                let ret = run_function(
                    module,
                    callee_name,
                    call_args,
                    host,
                    reg,
                    depth + 1,
                    config,
                )?;
                stack.push(ret);
            }
            Instr::CallBuiltin {
                package,
                name,
                argc,
            } => {
                if stack.len() < *argc {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Stack underflow on CallBuiltin",
                        function_name,
                        ip,
                    ));
                }
                let split = stack.len() - *argc;
                let call_args = stack.split_off(split);
                let ret = reg.call(host, package, name, call_args)?;
                stack.push(ret);
            }
            Instr::MakeArray(n) => arrays::make_array(&mut stack, *n, function_name, ip)?,
            Instr::MakeArrayRepeat(n) => {
                arrays::make_array_repeat(&mut stack, *n, function_name, ip)?
            }
            Instr::ArrayGet => arrays::array_get(&mut stack, function_name, ip)?,
            Instr::ArraySet => arrays::array_set(&mut stack, function_name, ip)?,
            Instr::ArraySetChain(depth) => {
                arrays::array_set_chain(&mut stack, *depth, function_name, ip)?
            }
            Instr::ArrayLen => arrays::array_len(&mut stack, function_name, ip)?,
            Instr::Return => {
                return Ok(stack.pop().unwrap_or(Value::Unit));
            }
        }
        ip += 1;
    }
    Ok(Value::Unit)
}

pub(super) fn err_at(kind: VmErrorKind, message: impl Into<String>, function: &str, ip: usize) -> VmError {
    let msg = message.into();
    VmError::new(kind, format!("{function}@{ip}: {msg}"))
}
