use crate::bytecode::{BytecodeModule, FunctionChunk, Value};
use crate::vm::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

pub(super) struct CallEnv<'a> {
    pub module: &'a BytecodeModule,
    pub fn_table: &'a [&'a FunctionChunk],
    pub globals: &'a mut Vec<Value>,
    pub host: &'a mut dyn BuiltinHost,
    pub reg: &'a BuiltinRegistry,
    pub opts: super::RunOptions,
}

pub(super) struct Site<'a> {
    pub function_name: &'a str,
    pub ip: usize,
}

fn take_call_args(stack: &mut Vec<Value>, argc: usize) -> Vec<Value> {
    match argc {
        0 => Vec::new(),
        1 => {
            // callers validate stack length before invoking this helper
            vec![stack.pop().expect("call arg stack underflow")]
        }
        2 => {
            let b = stack.pop().expect("call arg stack underflow");
            let a = stack.pop().expect("call arg stack underflow");
            vec![a, b]
        }
        _ => {
            let split = stack.len() - argc;
            stack.split_off(split)
        }
    }
}

pub(super) fn call(
    stack: &mut Vec<Value>,
    callee_name: &str,
    argc: usize,
    env: &mut CallEnv<'_>,
    site: Site<'_>,
) -> Result<(), VmError> {
    if stack.len() < argc {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on Call",
            site.function_name,
            site.ip,
        ));
    }
    let call_args = take_call_args(stack, argc);
    let Some(chunk) = env.module.functions.get(callee_name) else {
        return Err(super::err_at(
            VmErrorKind::UnknownFunction,
            format!("Unknown function `{callee_name}`"),
            site.function_name,
            site.ip,
        ));
    };
    let ret = super::run_chunk(
        &mut super::ExecEnv {
            module: env.module,
            fn_table: env.fn_table,
            globals: env.globals,
            host: env.host,
            reg: env.reg,
        },
        chunk,
        callee_name,
        call_args,
        super::RunOptions {
            depth: env.opts.depth + 1,
            config: env.opts.config,
        },
    )?;
    stack.push(ret);
    Ok(())
}

pub(super) fn call_idx(
    stack: &mut Vec<Value>,
    callee_idx: usize,
    argc: usize,
    env: &mut CallEnv<'_>,
    site: Site<'_>,
) -> Result<(), VmError> {
    if stack.len() < argc {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on CallIdx",
            site.function_name,
            site.ip,
        ));
    }
    let call_args = take_call_args(stack, argc);
    let Some(chunk) = env.fn_table.get(callee_idx).copied() else {
        return Err(super::err_at(
            VmErrorKind::UnknownFunction,
            format!("Invalid function index `{callee_idx}`"),
            site.function_name,
            site.ip,
        ));
    };
    let ret = super::run_chunk(
        &mut super::ExecEnv {
            module: env.module,
            fn_table: env.fn_table,
            globals: env.globals,
            host: env.host,
            reg: env.reg,
        },
        chunk,
        &chunk.name,
        call_args,
        super::RunOptions {
            depth: env.opts.depth + 1,
            config: env.opts.config,
        },
    )?;
    stack.push(ret);
    Ok(())
}

pub(super) fn call_value(
    stack: &mut Vec<Value>,
    argc: usize,
    env: &mut CallEnv<'_>,
    site: Site<'_>,
) -> Result<(), VmError> {
    if stack.len() < argc + 1 {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on CallValue",
            site.function_name,
            site.ip,
        ));
    }
    let call_args = take_call_args(stack, argc);
    let Some(callee) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "CallValue expects callable on stack",
            site.function_name,
            site.ip,
        ));
    };
    let Value::Function(callee_name) = callee else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "CallValue callee must be Function",
            site.function_name,
            site.ip,
        ));
    };
    let Some(chunk) = env.module.functions.get(&callee_name) else {
        return Err(super::err_at(
            VmErrorKind::UnknownFunction,
            format!("Unknown function `{callee_name}`"),
            site.function_name,
            site.ip,
        ));
    };
    let ret = super::run_chunk(
        &mut super::ExecEnv {
            module: env.module,
            fn_table: env.fn_table,
            globals: env.globals,
            host: env.host,
            reg: env.reg,
        },
        chunk,
        &callee_name,
        call_args,
        super::RunOptions {
            depth: env.opts.depth + 1,
            config: env.opts.config,
        },
    )?;
    stack.push(ret);
    Ok(())
}

pub(super) fn call_builtin(
    stack: &mut Vec<Value>,
    package: &str,
    name: &str,
    argc: usize,
    env: &mut CallEnv<'_>,
    site: Site<'_>,
) -> Result<(), VmError> {
    if stack.len() < argc {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on CallBuiltin",
            site.function_name,
            site.ip,
        ));
    }
    let call_args = take_call_args(stack, argc);
    let ret = env.reg.call(env.host, package, name, call_args)?;
    stack.push(ret);
    Ok(())
}

pub(super) fn call_method(
    stack: &mut Vec<Value>,
    method_name: &str,
    argc: usize,
    env: &mut CallEnv<'_>,
    site: Site<'_>,
) -> Result<(), VmError> {
    if stack.len() < argc + 1 {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on CallMethod",
            site.function_name,
            site.ip,
        ));
    }
    let mut call_args = take_call_args(stack, argc);
    let Some(receiver) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "CallMethod expects receiver",
            site.function_name,
            site.ip,
        ));
    };
    let mut callee_name = String::new();
    let chunk = {
        let Value::Struct { name, .. } = &receiver else {
            return Err(super::err_at(
                VmErrorKind::TypeMismatch,
                "CallMethod receiver must be Struct",
                site.function_name,
                site.ip,
            ));
        };
        callee_name.reserve("__impl_".len() + name.len() + 2 + method_name.len());
        callee_name.push_str("__impl_");
        callee_name.push_str(name);
        callee_name.push_str("__");
        callee_name.push_str(method_name);
        let Some(chunk) = env.module.functions.get(&callee_name) else {
            return Err(super::err_at(
                VmErrorKind::UnknownFunction,
                format!("Unknown method `{}` on struct `{}`", method_name, name),
                site.function_name,
                site.ip,
            ));
        };
        chunk
    };
    let mut full_args = Vec::with_capacity(argc + 1);
    full_args.push(receiver);
    full_args.append(&mut call_args);
    let ret = super::run_chunk(
        &mut super::ExecEnv {
            module: env.module,
            fn_table: env.fn_table,
            globals: env.globals,
            host: env.host,
            reg: env.reg,
        },
        chunk,
        &callee_name,
        full_args,
        super::RunOptions {
            depth: env.opts.depth + 1,
            config: env.opts.config,
        },
    )?;
    stack.push(ret);
    Ok(())
}
