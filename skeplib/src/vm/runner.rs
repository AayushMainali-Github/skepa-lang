//! VM interpreter loop and instruction dispatch.

mod arith;
mod arrays;
mod calls;
mod control_flow;
mod state;
mod structs;

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

    let mut state = state::VmState::new(chunk.locals_count, args);

    let mut ip = 0usize;
    while ip < chunk.code.len() {
        if config.trace {
            eprintln!("[trace] {}@{} {:?}", function_name, ip, chunk.code[ip]);
        }
        let mut call_env = calls::CallEnv {
            module,
            host,
            reg,
            depth,
            config,
        };
        match &chunk.code[ip] {
            Instr::LoadConst(v) => state.push_const(v.clone()),
            Instr::LoadLocal(slot) => state.load_local(*slot, function_name, ip)?,
            Instr::StoreLocal(slot) => state.store_local(*slot, function_name, ip)?,
            Instr::Pop => state.pop_discard(function_name, ip)?,
            Instr::NegInt => arith::neg(state.stack_mut(), function_name, ip)?,
            Instr::NotBool => arith::not_bool(state.stack_mut(), function_name, ip)?,
            Instr::Add => arith::add(state.stack_mut(), function_name, ip)?,
            Instr::SubInt
            | Instr::MulInt
            | Instr::DivInt
            | Instr::LtInt
            | Instr::LteInt
            | Instr::GtInt
            | Instr::GteInt => {
                arith::numeric_binop(state.stack_mut(), &chunk.code[ip], function_name, ip)?
            }
            Instr::ModInt => arith::mod_int(state.stack_mut(), function_name, ip)?,
            Instr::Eq => arith::eq(state.stack_mut(), function_name, ip)?,
            Instr::Neq => arith::neq(state.stack_mut(), function_name, ip)?,
            Instr::AndBool | Instr::OrBool => {
                arith::logical(state.stack_mut(), &chunk.code[ip], function_name, ip)?
            }
            Instr::Jump(target) => {
                ip = control_flow::jump(*target);
                continue;
            }
            Instr::JumpIfFalse(target) => {
                if let Some(next_ip) =
                    control_flow::jump_if_false(state.stack_mut(), *target, function_name, ip)?
                {
                    ip = next_ip;
                    continue;
                }
            }
            Instr::JumpIfTrue(target) => {
                if let Some(next_ip) =
                    control_flow::jump_if_true(state.stack_mut(), *target, function_name, ip)?
                {
                    ip = next_ip;
                    continue;
                }
            }
            Instr::Call {
                name: callee_name,
                argc,
            } => calls::call(
                state.stack_mut(),
                callee_name,
                *argc,
                &mut call_env,
                calls::Site { function_name, ip },
            )?,
            Instr::CallBuiltin {
                package,
                name,
                argc,
            } => calls::call_builtin(
                state.stack_mut(),
                package,
                name,
                *argc,
                &mut call_env,
                calls::Site { function_name, ip },
            )?,
            Instr::MakeArray(n) => arrays::make_array(state.stack_mut(), *n, function_name, ip)?,
            Instr::MakeArrayRepeat(n) => {
                arrays::make_array_repeat(state.stack_mut(), *n, function_name, ip)?
            }
            Instr::ArrayGet => arrays::array_get(state.stack_mut(), function_name, ip)?,
            Instr::ArraySet => arrays::array_set(state.stack_mut(), function_name, ip)?,
            Instr::ArraySetChain(depth) => {
                arrays::array_set_chain(state.stack_mut(), *depth, function_name, ip)?
            }
            Instr::ArrayLen => arrays::array_len(state.stack_mut(), function_name, ip)?,
            Instr::MakeStruct { name, fields } => {
                structs::make_struct(state.stack_mut(), name, fields, function_name, ip)?
            }
            Instr::StructGet(field) => {
                structs::struct_get(state.stack_mut(), field, function_name, ip)?
            }
            Instr::StructSetPath(path) => {
                structs::struct_set_path(state.stack_mut(), path, function_name, ip)?
            }
            Instr::Return => {
                return Ok(state.finish_return());
            }
        }
        ip += 1;
    }
    Ok(Value::Unit)
}

pub(super) fn err_at(
    kind: VmErrorKind,
    message: impl Into<String>,
    function: &str,
    ip: usize,
) -> VmError {
    let msg = message.into();
    VmError::new(kind, format!("{function}@{ip}: {msg}"))
}
