//! VM interpreter loop and instruction dispatch.

mod arith;
mod arrays;
mod calls;
mod control_flow;
mod state;
mod structs;

use crate::bytecode::{BytecodeModule, FunctionChunk, Instr, Value};

use super::{BuiltinHost, BuiltinRegistry, VmConfig, VmError, VmErrorKind};

#[derive(Clone, Copy)]
pub(super) struct RunOptions {
    pub depth: usize,
    pub config: VmConfig,
}

pub(super) struct ExecEnv<'a> {
    pub module: &'a BytecodeModule,
    pub fn_table: &'a [&'a FunctionChunk],
    pub globals: &'a mut Vec<Value>,
    pub host: &'a mut dyn BuiltinHost,
    pub reg: &'a BuiltinRegistry,
}

pub(super) fn run_function(
    env: &mut ExecEnv<'_>,
    function_name: &str,
    args: Vec<Value>,
    opts: RunOptions,
) -> Result<Value, VmError> {
    let Some(chunk) = env.module.functions.get(function_name) else {
        return Err(VmError::new(
            VmErrorKind::UnknownFunction,
            format!("Unknown function `{function_name}`"),
        ));
    };
    run_chunk(env, chunk, function_name, args, opts)
}

pub(super) fn run_chunk(
    env: &mut ExecEnv<'_>,
    chunk: &FunctionChunk,
    function_name: &str,
    args: Vec<Value>,
    opts: RunOptions,
) -> Result<Value, VmError> {
    let _chunk_timer = super::profiler::ScopedTimer::new(super::profiler::Event::RunChunk);
    let profile_ops = std::env::var_os("SKEPA_VM_PROFILE_OPS").is_some();
    let mut prof_load_local = 0u64;
    let mut prof_store_local = 0u64;
    let mut prof_add = 0u64;
    let mut prof_mod = 0u64;
    let mut prof_lte = 0u64;
    let mut prof_jump = 0u64;
    let mut prof_jump_if_false = 0u64;
    if opts.depth >= opts.config.max_call_depth {
        return Err(VmError::new(
            VmErrorKind::StackOverflow,
            format!("Call stack limit exceeded ({})", opts.config.max_call_depth),
        ));
    }
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

    let stack_capacity_hint = (chunk.code.len() / 4).clamp(8, 256);
    let mut state = state::VmState::new(chunk.locals_count, stack_capacity_hint, args);

    let mut ip = 0usize;
    while ip < chunk.code.len() {
        if opts.config.trace {
            eprintln!("[trace] {}@{} {:?}", function_name, ip, chunk.code[ip]);
        }
        super::profiler::record_instr(&chunk.code[ip]);
        match &chunk.code[ip] {
            Instr::LoadConst(v) => state.push_const(v.clone()),
            Instr::LoadLocal(slot) => {
                if profile_ops {
                    prof_load_local += 1;
                }
                state.load_local(*slot, function_name, ip)?
            }
            Instr::StoreLocal(slot) => {
                if profile_ops {
                    prof_store_local += 1;
                }
                state.store_local(*slot, function_name, ip)?
            }
            Instr::LoadGlobal(slot) => {
                let Some(v) = env.globals.get(*slot).cloned() else {
                    return Err(err_at(
                        VmErrorKind::InvalidLocal,
                        format!("Invalid global slot {slot}"),
                        function_name,
                        ip,
                    ));
                };
                state.push_const(v);
            }
            Instr::StoreGlobal(slot) => {
                let Some(v) = state.stack_mut().pop() else {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Stack underflow on StoreGlobal",
                        function_name,
                        ip,
                    ));
                };
                if *slot >= env.globals.len() {
                    env.globals.resize(*slot + 1, Value::Unit);
                }
                env.globals[*slot] = v;
            }
            Instr::Pop => state.pop_discard(function_name, ip)?,
            Instr::NegInt => arith::neg(state.stack_mut(), function_name, ip)?,
            Instr::NotBool => arith::not_bool(state.stack_mut(), function_name, ip)?,
            Instr::Add => {
                if profile_ops {
                    prof_add += 1;
                }
                let stack = state.stack_mut();
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
                    (l, r) => {
                        stack.push(l);
                        stack.push(r);
                        arith::add(stack, function_name, ip)?
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
                if matches!(&chunk.code[ip], Instr::LteInt) {
                    if profile_ops {
                        prof_lte += 1;
                    }
                    let stack = state.stack_mut();
                    let Some(r) = stack.pop() else {
                        return Err(err_at(
                            VmErrorKind::StackUnderflow,
                            "int binary op expects rhs",
                            function_name,
                            ip,
                        ));
                    };
                    let Some(l) = stack.pop() else {
                        stack.push(r);
                        return Err(err_at(
                            VmErrorKind::StackUnderflow,
                            "int binary op expects lhs",
                            function_name,
                            ip,
                        ));
                    };
                    match (l, r) {
                        (Value::Int(l), Value::Int(r)) => stack.push(Value::Bool(l <= r)),
                        (l, r) => {
                            stack.push(l);
                            stack.push(r);
                            arith::numeric_binop(stack, &chunk.code[ip], function_name, ip)?
                        }
                    }
                } else {
                    arith::numeric_binop(state.stack_mut(), &chunk.code[ip], function_name, ip)?
                }
            }
            Instr::ModInt => {
                if profile_ops {
                    prof_mod += 1;
                }
                let stack = state.stack_mut();
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
                        arith::mod_int(stack, function_name, ip)?
                    }
                }
            }
            Instr::Eq => arith::eq(state.stack_mut(), function_name, ip)?,
            Instr::Neq => arith::neq(state.stack_mut(), function_name, ip)?,
            Instr::AndBool | Instr::OrBool => {
                arith::logical(state.stack_mut(), &chunk.code[ip], function_name, ip)?
            }
            Instr::Jump(target) => {
                if profile_ops {
                    prof_jump += 1;
                }
                ip = control_flow::jump(*target);
                continue;
            }
            Instr::JumpIfFalse(target) => {
                if profile_ops {
                    prof_jump_if_false += 1;
                }
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
                &mut calls::CallEnv {
                    module: env.module,
                    fn_table: env.fn_table,
                    globals: env.globals,
                    host: env.host,
                    reg: env.reg,
                    opts,
                },
                calls::Site { function_name, ip },
            )?,
            Instr::CallIdx { idx, argc } => calls::call_idx(
                state.stack_mut(),
                *idx,
                *argc,
                &mut calls::CallEnv {
                    module: env.module,
                    fn_table: env.fn_table,
                    globals: env.globals,
                    host: env.host,
                    reg: env.reg,
                    opts,
                },
                calls::Site { function_name, ip },
            )?,
            Instr::CallValue { argc } => calls::call_value(
                state.stack_mut(),
                *argc,
                &mut calls::CallEnv {
                    module: env.module,
                    fn_table: env.fn_table,
                    globals: env.globals,
                    host: env.host,
                    reg: env.reg,
                    opts,
                },
                calls::Site { function_name, ip },
            )?,
            Instr::CallMethod {
                name: method_name,
                argc,
            } => calls::call_method(
                state.stack_mut(),
                method_name,
                *argc,
                &mut calls::CallEnv {
                    module: env.module,
                    fn_table: env.fn_table,
                    globals: env.globals,
                    host: env.host,
                    reg: env.reg,
                    opts,
                },
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
                &mut calls::CallEnv {
                    module: env.module,
                    fn_table: env.fn_table,
                    globals: env.globals,
                    host: env.host,
                    reg: env.reg,
                    opts,
                },
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
                if profile_ops && opts.depth == 0 {
                    eprintln!(
                        "[vm-prof] {function_name}: LoadLocal={prof_load_local} StoreLocal={prof_store_local} Add={prof_add} ModInt={prof_mod} LteInt={prof_lte} Jump={prof_jump} JumpIfFalse={prof_jump_if_false}"
                    );
                }
                return Ok(state.finish_return());
            }
        }
        ip += 1;
    }
    if profile_ops && opts.depth == 0 {
        eprintln!(
            "[vm-prof] {function_name}: LoadLocal={prof_load_local} StoreLocal={prof_store_local} Add={prof_add} ModInt={prof_mod} LteInt={prof_lte} Jump={prof_jump} JumpIfFalse={prof_jump_if_false}"
        );
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
