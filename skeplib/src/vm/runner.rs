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

    let mut locals_pool: Vec<Vec<Value>> = Vec::new();
    let mut stack_pool: Vec<Vec<Value>> = Vec::new();
    let mut frames = Vec::with_capacity((opts.depth + 1).clamp(1, 64));
    let mut root_frame = state::CallFrame::with_storage(
        chunk,
        function_name,
        state::acquire_storage(
            chunk.locals_count,
            stack_capacity_hint(chunk),
            &mut locals_pool,
            &mut stack_pool,
        ),
    );
    root_frame.locals.extend(args);
    if root_frame.locals.len() < chunk.locals_count {
        root_frame.locals.resize(chunk.locals_count, Value::Unit);
    }
    frames.push(root_frame);

    loop {
        let current_depth = frames.len();
        let Some(frame) = frames.last_mut() else {
            return Ok(Value::Unit);
        };
        if frame.ip >= frame.chunk.code.len() {
            let ret = Value::Unit;
            if let Some(storage) = frames.pop().map(state::CallFrame::into_storage) {
                locals_pool.push(storage.locals);
                stack_pool.push(storage.stack);
            }
            if let Some(parent) = frames.last_mut() {
                parent.stack.push(ret);
                continue;
            }
            return Ok(ret);
        }
        let function_name = frame.function_name;
        let ip = frame.ip;
        let instr = &frame.chunk.code[ip];
        if opts.config.trace {
            eprintln!("[trace] {}@{} {:?}", function_name, ip, instr);
        }
        if handle_hot_instr(frame, instr, function_name, ip)? {
            continue;
        }
        match instr {
            Instr::LoadConst(_)
            | Instr::LoadLocal(_)
            | Instr::StoreLocal(_)
            | Instr::AddLocalToLocal { .. }
            | Instr::AddConstToLocal { .. } => unreachable!(),
            Instr::LoadGlobal(slot) => {
                let Some(v) = env.globals.get(*slot).cloned() else {
                    return Err(err_at(
                        VmErrorKind::InvalidLocal,
                        format!("Invalid global slot {slot}"),
                        function_name,
                        ip,
                    ));
                };
                frame.stack.push(v);
            }
            Instr::StoreGlobal(slot) => {
                let Some(v) = frame.stack.pop() else {
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
            Instr::Pop => {
                if frame.stack.pop().is_none() {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Stack underflow on Pop",
                        function_name,
                        ip,
                    ));
                }
            }
            Instr::NegInt => arith::neg(&mut frame.stack, function_name, ip)?,
            Instr::NotBool => arith::not_bool(&mut frame.stack, function_name, ip)?,
            Instr::Add => unreachable!(),
            Instr::SubInt | Instr::MulInt | Instr::DivInt | Instr::GtInt | Instr::GteInt => {
                arith::numeric_binop(&mut frame.stack, instr, function_name, ip)?
            }
            Instr::LtInt => unreachable!(),
            Instr::LteInt => unreachable!(),
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
                        arith::mod_int(stack, function_name, ip)?
                    }
                }
            }
            Instr::Eq => arith::eq(&mut frame.stack, function_name, ip)?,
            Instr::Neq => arith::neq(&mut frame.stack, function_name, ip)?,
            Instr::AndBool | Instr::OrBool => {
                arith::logical(&mut frame.stack, instr, function_name, ip)?
            }
            Instr::Jump(_) | Instr::JumpIfFalse(_) | Instr::JumpIfLocalLtConst { .. } => {
                unreachable!()
            }
            Instr::JumpIfTrue(target) => {
                if let Some(next_ip) =
                    control_flow::jump_if_true(&mut frame.stack, *target, function_name, ip)?
                {
                    frame.ip = next_ip;
                    continue;
                }
            }
            Instr::Call {
                name: callee_name,
                argc,
            } => {
                if frame.stack.len() < *argc {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Stack underflow on Call",
                        function_name,
                        ip,
                    ));
                }
                let callee_chunk = calls::resolve_chunk(
                    env.module,
                    callee_name,
                    calls::Site { function_name, ip },
                )?;
                if current_depth + opts.depth >= opts.config.max_call_depth {
                    return Err(VmError::new(
                        VmErrorKind::StackOverflow,
                        format!("Call stack limit exceeded ({})", opts.config.max_call_depth),
                    ));
                }
                if *argc != callee_chunk.param_count {
                    return Err(VmError::new(
                        VmErrorKind::ArityMismatch,
                        format!(
                            "Function `{}` arity mismatch: expected {}, got {}",
                            callee_name, callee_chunk.param_count, argc
                        ),
                    ));
                }
                let new_frame = {
                    frame.ip += 1;
                    push_call_frame(
                        frame,
                        callee_chunk,
                        *argc,
                        None,
                        &mut locals_pool,
                        &mut stack_pool,
                    )
                };
                frames.push(new_frame);
                continue;
            }
            Instr::CallIdx { idx, argc } => {
                let new_frame = call_idx_fast(
                    frame,
                    IndexedCallCtx {
                        fn_table: env.fn_table,
                        idx: *idx,
                        argc: *argc,
                        current_depth,
                        opts,
                        function_name,
                        ip,
                        locals_pool: &mut locals_pool,
                        stack_pool: &mut stack_pool,
                    },
                )?;
                frames.push(new_frame);
                continue;
            }
            Instr::CallIdxAddConst(rhs) => {
                let Some(value) = frame.stack.pop() else {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Stack underflow on CallIdxAddConst",
                        function_name,
                        ip,
                    ));
                };
                match value {
                    Value::Int(lhs) => frame.stack.push(Value::Int(lhs + rhs)),
                    _ => {
                        return Err(err_at(
                            VmErrorKind::TypeMismatch,
                            "CallIdxAddConst expects Int argument",
                            function_name,
                            ip,
                        ));
                    }
                }
            }
            Instr::CallValue { argc } => {
                if frame.stack.len() < *argc + 1 {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Stack underflow on CallValue",
                        function_name,
                        ip,
                    ));
                }
                let callee_index = frame.stack.len() - *argc - 1;
                let callee = frame.stack.remove(callee_index);
                let callee_chunk = calls::resolve_function_value(
                    env.module,
                    env.fn_table,
                    callee,
                    calls::Site { function_name, ip },
                )?;
                if current_depth + opts.depth >= opts.config.max_call_depth {
                    return Err(VmError::new(
                        VmErrorKind::StackOverflow,
                        format!("Call stack limit exceeded ({})", opts.config.max_call_depth),
                    ));
                }
                if *argc != callee_chunk.param_count {
                    return Err(VmError::new(
                        VmErrorKind::ArityMismatch,
                        format!(
                            "Function `{}` arity mismatch: expected {}, got {}",
                            callee_chunk.name, callee_chunk.param_count, argc
                        ),
                    ));
                }
                let new_frame = {
                    frame.ip += 1;
                    push_call_frame(
                        frame,
                        callee_chunk,
                        *argc,
                        None,
                        &mut locals_pool,
                        &mut stack_pool,
                    )
                };
                frames.push(new_frame);
                continue;
            }
            Instr::CallMethod {
                name: method_name,
                argc,
            } => {
                if frame.stack.len() < *argc + 1 {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Stack underflow on CallMethod",
                        function_name,
                        ip,
                    ));
                }
                let receiver_index = frame.stack.len() - *argc - 1;
                let receiver = frame.stack.remove(receiver_index);
                let callee_chunk = calls::resolve_method(
                    env.module,
                    env.fn_table,
                    &receiver,
                    method_name,
                    calls::Site { function_name, ip },
                )?;
                if current_depth + opts.depth >= opts.config.max_call_depth {
                    return Err(VmError::new(
                        VmErrorKind::StackOverflow,
                        format!("Call stack limit exceeded ({})", opts.config.max_call_depth),
                    ));
                }
                if *argc + 1 != callee_chunk.param_count {
                    return Err(VmError::new(
                        VmErrorKind::ArityMismatch,
                        format!(
                            "Function `{}` arity mismatch: expected {}, got {}",
                            callee_chunk.name,
                            callee_chunk.param_count,
                            argc + 1
                        ),
                    ));
                }
                let new_frame = {
                    frame.ip += 1;
                    push_call_frame(
                        frame,
                        callee_chunk,
                        *argc,
                        Some(receiver),
                        &mut locals_pool,
                        &mut stack_pool,
                    )
                };
                frames.push(new_frame);
                continue;
            }
            Instr::CallMethodId { id, argc } => {
                if frame.stack.len() < *argc + 1 {
                    return Err(err_at(
                        VmErrorKind::StackUnderflow,
                        "Stack underflow on CallMethodId",
                        function_name,
                        ip,
                    ));
                }
                let receiver_index = frame.stack.len() - *argc - 1;
                let receiver = frame.stack.remove(receiver_index);
                let callee_chunk = calls::resolve_method_id(
                    env.module,
                    env.fn_table,
                    &receiver,
                    *id,
                    calls::Site { function_name, ip },
                )?;
                if current_depth + opts.depth >= opts.config.max_call_depth {
                    return Err(VmError::new(
                        VmErrorKind::StackOverflow,
                        format!("Call stack limit exceeded ({})", opts.config.max_call_depth),
                    ));
                }
                if *argc + 1 != callee_chunk.param_count {
                    return Err(VmError::new(
                        VmErrorKind::ArityMismatch,
                        format!(
                            "Function `{}` arity mismatch: expected {}, got {}",
                            callee_chunk.name,
                            callee_chunk.param_count,
                            argc + 1
                        ),
                    ));
                }
                let new_frame = {
                    frame.ip += 1;
                    push_call_frame(
                        frame,
                        callee_chunk,
                        *argc,
                        Some(receiver),
                        &mut locals_pool,
                        &mut stack_pool,
                    )
                };
                frames.push(new_frame);
                continue;
            }
            Instr::CallBuiltin {
                package,
                name,
                argc,
            } => calls::call_builtin(
                &mut frame.stack,
                package,
                name,
                *argc,
                &mut calls::CallEnv {
                    host: env.host,
                    reg: env.reg,
                },
                calls::Site { function_name, ip },
            )?,
            Instr::CallBuiltinId { id, argc } => calls::call_builtin_id(
                &mut frame.stack,
                *id,
                *argc,
                &mut calls::CallEnv {
                    host: env.host,
                    reg: env.reg,
                },
                calls::Site { function_name, ip },
            )?,
            Instr::MakeArray(n) => arrays::make_array(&mut frame.stack, *n, function_name, ip)?,
            Instr::MakeArrayRepeat(n) => {
                arrays::make_array_repeat(&mut frame.stack, *n, function_name, ip)?
            }
            Instr::ArrayGet => arrays::array_get(&mut frame.stack, function_name, ip)?,
            Instr::ArraySet => arrays::array_set(&mut frame.stack, function_name, ip)?,
            Instr::ArraySetLocal(slot) => arrays::array_set_local(frame, *slot, function_name, ip)?,
            Instr::ArrayIncLocal(slot) => arrays::array_inc_local(frame, *slot, function_name, ip)?,
            Instr::ArraySetChain(depth) => {
                arrays::array_set_chain(&mut frame.stack, *depth, function_name, ip)?
            }
            Instr::ArrayLen => arrays::array_len(&mut frame.stack, function_name, ip)?,
            Instr::MakeStruct { name, fields } => {
                structs::make_struct(&mut frame.stack, name, fields, function_name, ip)?
            }
            Instr::MakeStructId { id } => {
                structs::make_struct_id(&mut frame.stack, env.module, *id, function_name, ip)?
            }
            Instr::StructGet(field) => {
                structs::struct_get(&mut frame.stack, field, function_name, ip)?
            }
            Instr::StructGetSlot(slot) => {
                structs::struct_get_slot(&mut frame.stack, *slot, function_name, ip)?
            }
            Instr::StructSetPath(path) => {
                structs::struct_set_path(&mut frame.stack, path, function_name, ip)?
            }
            Instr::StructSetPathSlots(path) => {
                structs::struct_set_path_slots(&mut frame.stack, path, function_name, ip)?
            }
            Instr::Return => {
                let ret = frame.stack.pop().unwrap_or(Value::Unit);
                if let Some(storage) = frames.pop().map(state::CallFrame::into_storage) {
                    locals_pool.push(storage.locals);
                    stack_pool.push(storage.stack);
                }
                if let Some(parent) = frames.last_mut() {
                    parent.stack.push(ret);
                    continue;
                }
                return Ok(ret);
            }
        }
        if let Some(frame) = frames.last_mut() {
            frame.ip += 1;
        }
    }
}

#[inline(always)]
fn stack_capacity_hint(chunk: &FunctionChunk) -> usize {
    (chunk.code.len() / 4).clamp(8, 256)
}

#[inline(always)]
fn handle_hot_instr(
    frame: &mut state::CallFrame<'_>,
    instr: &Instr,
    function_name: &str,
    ip: usize,
) -> Result<bool, VmError> {
    match instr {
        Instr::LoadConst(v) => {
            frame.stack.push(v.clone());
            frame.ip += 1;
            Ok(true)
        }
        Instr::LoadLocal(slot) => {
            let Some(v) = frame.read_local_cloned(*slot) else {
                return Err(invalid_local_slot(function_name, ip, *slot));
            };
            frame.stack.push(v);
            frame.ip += 1;
            Ok(true)
        }
        Instr::StoreLocal(slot) => {
            let Some(v) = frame.stack.pop() else {
                return Err(err_at(
                    VmErrorKind::StackUnderflow,
                    "Stack underflow on StoreLocal",
                    function_name,
                    ip,
                ));
            };
            if !frame.write_local_fast(*slot, v) {
                return Err(invalid_local_slot(function_name, ip, *slot));
            }
            frame.ip += 1;
            Ok(true)
        }
        Instr::AddLocalToLocal { dst, src } => {
            let Some(lhs) = frame.read_local_cloned(*dst) else {
                return Err(invalid_local_slot(function_name, ip, *dst));
            };
            let Some(rhs) = frame.read_local_cloned(*src) else {
                return Err(invalid_local_slot(function_name, ip, *src));
            };
            match (lhs, rhs) {
                (Value::Int(lhs), Value::Int(rhs)) => {
                    if !frame.write_local_fast(*dst, Value::Int(lhs + rhs)) {
                        return Err(invalid_local_slot(function_name, ip, *dst));
                    }
                    frame.ip += 1;
                    Ok(true)
                }
                _ => Ok(false),
            }
        }
        Instr::AddConstToLocal { slot, rhs } => {
            let Some(value) = frame.read_local_cloned(*slot) else {
                return Err(invalid_local_slot(function_name, ip, *slot));
            };
            match value {
                Value::Int(lhs) => {
                    if !frame.write_local_fast(*slot, Value::Int(lhs + rhs)) {
                        return Err(invalid_local_slot(function_name, ip, *slot));
                    }
                    frame.ip += 1;
                    Ok(true)
                }
                _ => Ok(false),
            }
        }
        Instr::Add => {
            let Some((l, r)) = frame.pop2() else {
                return Err(err_at(
                    VmErrorKind::StackUnderflow,
                    "Add expects lhs/rhs",
                    function_name,
                    ip,
                ));
            };
            let stack = &mut frame.stack;
            match (l, r) {
                (Value::Int(a), Value::Int(b)) => stack.push(Value::Int(a + b)),
                (l, r) => {
                    stack.push(l);
                    stack.push(r);
                    arith::add(stack, function_name, ip)?;
                }
            }
            frame.ip += 1;
            Ok(true)
        }
        Instr::LteInt => {
            let Some((l, r)) = frame.pop2() else {
                return Err(err_at(
                    VmErrorKind::StackUnderflow,
                    "int binary op expects lhs/rhs",
                    function_name,
                    ip,
                ));
            };
            let stack = &mut frame.stack;
            match (l, r) {
                (Value::Int(l), Value::Int(r)) => stack.push(Value::Bool(l <= r)),
                (l, r) => {
                    stack.push(l);
                    stack.push(r);
                    arith::numeric_binop(stack, instr, function_name, ip)?;
                }
            }
            frame.ip += 1;
            Ok(true)
        }
        Instr::LtInt => {
            let Some((l, r)) = frame.pop2() else {
                return Err(err_at(
                    VmErrorKind::StackUnderflow,
                    "int binary op expects lhs/rhs",
                    function_name,
                    ip,
                ));
            };
            let stack = &mut frame.stack;
            match (l, r) {
                (Value::Int(l), Value::Int(r)) => stack.push(Value::Bool(l < r)),
                (l, r) => {
                    stack.push(l);
                    stack.push(r);
                    arith::numeric_binop(stack, instr, function_name, ip)?;
                }
            }
            frame.ip += 1;
            Ok(true)
        }
        Instr::Jump(target) => {
            frame.ip = control_flow::jump(*target);
            Ok(true)
        }
        Instr::JumpIfLocalLtConst { slot, rhs, target } => {
            let Some(value) = frame.locals.get(*slot) else {
                return Err(invalid_local_slot(function_name, ip, *slot));
            };
            match value {
                Value::Int(current) => {
                    if *current < *rhs {
                        frame.ip += 1;
                    } else {
                        frame.ip = *target;
                    }
                    Ok(true)
                }
                _ => Err(err_at(
                    VmErrorKind::TypeMismatch,
                    "JumpIfLocalLtConst expects Int local",
                    function_name,
                    ip,
                )),
            }
        }
        Instr::JumpIfFalse(target) => {
            let Some(cond) = frame.stack.pop() else {
                return Err(err_at(
                    VmErrorKind::TypeMismatch,
                    "JumpIfFalse expects Bool",
                    function_name,
                    ip,
                ));
            };
            match cond {
                Value::Bool(false) => frame.ip = *target,
                Value::Bool(true) => frame.ip += 1,
                _ => {
                    return Err(err_at(
                        VmErrorKind::TypeMismatch,
                        "JumpIfFalse expects Bool",
                        function_name,
                        ip,
                    ));
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[inline(always)]
fn call_idx_fast<'a>(
    frame: &mut state::CallFrame<'_>,
    ctx: IndexedCallCtx<'a, '_>,
) -> Result<state::CallFrame<'a>, VmError> {
    if frame.stack.len() < ctx.argc {
        return Err(err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on CallIdx",
            ctx.function_name,
            ctx.ip,
        ));
    }
    let Some(callee_chunk) = ctx.fn_table.get(ctx.idx).copied() else {
        return Err(err_at(
            VmErrorKind::UnknownFunction,
            format!("Invalid function index `{}`", ctx.idx),
            ctx.function_name,
            ctx.ip,
        ));
    };
    if ctx.current_depth + ctx.opts.depth >= ctx.opts.config.max_call_depth {
        return Err(VmError::new(
            VmErrorKind::StackOverflow,
            format!(
                "Call stack limit exceeded ({})",
                ctx.opts.config.max_call_depth
            ),
        ));
    }
    if ctx.argc != callee_chunk.param_count {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            format!(
                "Function `{}` arity mismatch: expected {}, got {}",
                callee_chunk.name, callee_chunk.param_count, ctx.argc
            ),
        ));
    }
    frame.ip += 1;
    Ok(push_call_frame(
        frame,
        callee_chunk,
        ctx.argc,
        None,
        ctx.locals_pool,
        ctx.stack_pool,
    ))
}

struct IndexedCallCtx<'a, 'b> {
    fn_table: &'a [&'a FunctionChunk],
    idx: usize,
    argc: usize,
    current_depth: usize,
    opts: RunOptions,
    function_name: &'b str,
    ip: usize,
    locals_pool: &'b mut Vec<Vec<Value>>,
    stack_pool: &'b mut Vec<Vec<Value>>,
}

fn push_call_frame<'a>(
    caller: &mut state::CallFrame<'_>,
    callee_chunk: &'a FunctionChunk,
    argc: usize,
    receiver: Option<Value>,
    locals_pool: &mut Vec<Vec<Value>>,
    stack_pool: &mut Vec<Vec<Value>>,
) -> state::CallFrame<'a> {
    let mut new_frame = state::CallFrame::with_storage(
        callee_chunk,
        &callee_chunk.name,
        state::acquire_storage(
            callee_chunk.locals_count,
            stack_capacity_hint(callee_chunk),
            locals_pool,
            stack_pool,
        ),
    );
    if let Some(receiver) = receiver {
        new_frame.locals.push(receiver);
    }
    let split = caller.stack.len() - argc;
    new_frame.locals.extend(caller.stack.split_off(split));
    if new_frame.locals.len() < callee_chunk.locals_count {
        new_frame
            .locals
            .resize(callee_chunk.locals_count, Value::Unit);
    }
    new_frame
}

#[cold]
fn invalid_local_slot(function_name: &str, ip: usize, slot: usize) -> VmError {
    err_at(
        VmErrorKind::InvalidLocal,
        format!("Invalid local slot {slot}"),
        function_name,
        ip,
    )
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
