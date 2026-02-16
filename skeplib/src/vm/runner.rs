mod arrays;
mod arith;
mod calls;
mod control_flow;

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
            Instr::NegInt => arith::neg(&mut stack, function_name, ip)?,
            Instr::NotBool => arith::not_bool(&mut stack, function_name, ip)?,
            Instr::Add => arith::add(&mut stack, function_name, ip)?,
            Instr::SubInt
            | Instr::MulInt
            | Instr::DivInt
            | Instr::LtInt
            | Instr::LteInt
            | Instr::GtInt
            | Instr::GteInt => {
                arith::numeric_binop(&mut stack, &chunk.code[ip], function_name, ip)?
            }
            Instr::ModInt => arith::mod_int(&mut stack, function_name, ip)?,
            Instr::Eq => arith::eq(&mut stack, function_name, ip)?,
            Instr::Neq => arith::neq(&mut stack, function_name, ip)?,
            Instr::AndBool | Instr::OrBool => {
                arith::logical(&mut stack, &chunk.code[ip], function_name, ip)?
            }
            Instr::Jump(target) => {
                ip = control_flow::jump(*target);
                continue;
            }
            Instr::JumpIfFalse(target) => {
                if let Some(next_ip) =
                    control_flow::jump_if_false(&mut stack, *target, function_name, ip)?
                {
                    ip = next_ip;
                    continue;
                }
            }
            Instr::JumpIfTrue(target) => {
                if let Some(next_ip) =
                    control_flow::jump_if_true(&mut stack, *target, function_name, ip)?
                {
                    ip = next_ip;
                    continue;
                }
            }
            Instr::Call { name: callee_name, argc } => calls::call(
                &mut stack,
                module,
                callee_name,
                *argc,
                host,
                reg,
                depth,
                config,
                function_name,
                ip,
            )?,
            Instr::CallBuiltin { package, name, argc } => calls::call_builtin(
                &mut stack,
                host,
                reg,
                package,
                name,
                *argc,
                function_name,
                ip,
            )?,
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
