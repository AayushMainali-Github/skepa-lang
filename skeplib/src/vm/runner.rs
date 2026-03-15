//! VM interpreter loop and instruction dispatch.

mod arith;
mod arrays;
mod calls;
mod control_flow;
mod core;
mod frame;
mod hot;
mod state;
mod strings;
mod structs;

use crate::bytecode::{BytecodeModule, FunctionChunk, Instr, IntLocalConstOp, Value};

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
        if hot::handle_hot_instr(frame, instr, function_name, ip)? {
            continue;
        }
        match instr {
            Instr::LoadConst(_)
            | Instr::LoadLocal(_)
            | Instr::StoreLocal(_)
            | Instr::AddLocalToLocal { .. }
            | Instr::AddConstToLocal { .. }
            | Instr::IntLocalConstOp { .. }
            | Instr::IntLocalConstOpToLocal { .. }
            | Instr::IntStackOpToLocal { .. }
            | Instr::IntStackConstOp { .. }
            | Instr::IntStackConstOpToLocal { .. }
            | Instr::IntLocalLocalOpToLocal { .. } => unreachable!(),
            Instr::LoadGlobal(slot) => {
                core::load_global(env.globals, &mut frame.stack, *slot, function_name, ip)?
            }
            Instr::StoreGlobal(slot) => {
                core::store_global(env.globals, &mut frame.stack, *slot, function_name, ip)?
            }
            Instr::Pop => core::pop(&mut frame.stack, function_name, ip)?,
            Instr::NegInt => arith::neg(&mut frame.stack, function_name, ip)?,
            Instr::NotBool => arith::not_bool(&mut frame.stack, function_name, ip)?,
            Instr::Add => unreachable!(),
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
                            IntLocalConstOp::Add => {
                                arith::add(&mut frame.stack, function_name, ip)?
                            }
                            IntLocalConstOp::Sub | IntLocalConstOp::Mul | IntLocalConstOp::Div => {
                                let generic_instr = match op {
                                    IntLocalConstOp::Sub => &Instr::SubInt,
                                    IntLocalConstOp::Mul => &Instr::MulInt,
                                    IntLocalConstOp::Div => &Instr::DivInt,
                                    IntLocalConstOp::Add | IntLocalConstOp::Mod => unreachable!(),
                                };
                                arith::numeric_binop(
                                    &mut frame.stack,
                                    generic_instr,
                                    function_name,
                                    ip,
                                )?
                            }
                            IntLocalConstOp::Mod => {
                                arith::mod_int(&mut frame.stack, function_name, ip)?
                            }
                        }
                    }
                }
            }
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
            Instr::Call { .. }
            | Instr::CallIdx { .. }
            | Instr::CallIdxAddConst(_)
            | Instr::CallIdxStructFieldAdd(_)
            | Instr::CallValue { .. }
            | Instr::CallMethod { .. }
            | Instr::CallMethodId { .. }
            | Instr::CallBuiltin { .. }
            | Instr::CallBuiltinId { .. } => {
                if let Some(new_frame) = calls::handle_call_instr(
                    frame,
                    instr,
                    calls::DispatchCtx {
                        module: env.module,
                        fn_table: env.fn_table,
                        host: env.host,
                        reg: env.reg,
                        current_depth,
                        opts: calls::CallOptions {
                            depth: opts.depth,
                            config: opts.config,
                        },
                        function_name,
                        ip,
                        locals_pool: &mut locals_pool,
                        stack_pool: &mut stack_pool,
                    },
                )? {
                    frames.push(new_frame);
                    continue;
                }
            }
            Instr::StrLen
            | Instr::StrLenLocal(_)
            | Instr::StrIndexOfConst(_)
            | Instr::StrIndexOfLocalConst { .. }
            | Instr::StrSliceConst { .. }
            | Instr::StrSliceLocalConst { .. }
            | Instr::StrContainsConst(_)
            | Instr::StrContainsLocalConst { .. } => {
                unreachable!("handled in hot instruction path")
            }
            Instr::MakeArray(n) => arrays::make_array(&mut frame.stack, *n, function_name, ip)?,
            Instr::MakeArrayRepeat(n) => {
                arrays::make_array_repeat(&mut frame.stack, *n, function_name, ip)?
            }
            Instr::ArrayGet => arrays::array_get(&mut frame.stack, function_name, ip)?,
            Instr::ArrayGetLocal(slot) => arrays::array_get_local(frame, *slot, function_name, ip)?,
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
            Instr::StructGetLocalSlot { slot, field_slot } => structs::struct_get_local_slot(
                &frame.locals,
                &mut frame.stack,
                *slot,
                *field_slot,
                function_name,
                ip,
            )?,
            Instr::StructGetLocalSlotAddToLocal {
                struct_slot,
                field_slot,
                dst,
            } => structs::struct_get_local_slot_add_to_local(
                &mut frame.locals,
                *struct_slot,
                *field_slot,
                *dst,
                function_name,
                ip,
            )?,
            Instr::StructFieldAddMulFieldModLocalToLocal {
                struct_slot,
                arg_slot,
                arg_op,
                arg_rhs,
                lhs_field_slot,
                rhs_field_slot,
                mul,
                modulo,
                dst,
            } => structs::struct_field_add_mul_field_mod_local_to_local(
                &mut frame.locals,
                *struct_slot,
                *arg_slot,
                *arg_op,
                *arg_rhs,
                *lhs_field_slot,
                *rhs_field_slot,
                *mul,
                *modulo,
                *dst,
                function_name,
                ip,
            )?,
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
                if let Some(ret) =
                    core::finish_return(&mut frames, &mut locals_pool, &mut stack_pool)?
                {
                    return Ok(ret);
                }
                continue;
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
