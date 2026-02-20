use crate::bytecode::{BytecodeModule, Value};
use crate::vm::{BuiltinHost, BuiltinRegistry, VmConfig, VmError, VmErrorKind};

pub(super) struct CallEnv<'a> {
    pub module: &'a BytecodeModule,
    pub host: &'a mut dyn BuiltinHost,
    pub reg: &'a BuiltinRegistry,
    pub depth: usize,
    pub config: VmConfig,
}

pub(super) struct Site<'a> {
    pub function_name: &'a str,
    pub ip: usize,
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
    let split = stack.len() - argc;
    let call_args = stack.split_off(split);
    let ret = super::run_function(
        env.module,
        callee_name,
        call_args,
        env.host,
        env.reg,
        env.depth + 1,
        env.config,
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
    let split = stack.len() - argc;
    let call_args = stack.split_off(split);
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
    let ret = super::run_function(
        env.module,
        &callee_name,
        call_args,
        env.host,
        env.reg,
        env.depth + 1,
        env.config,
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
    let split = stack.len() - argc;
    let call_args = stack.split_off(split);
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
    let split = stack.len() - argc;
    let mut call_args = stack.split_off(split);
    let Some(receiver) = stack.pop() else {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "CallMethod expects receiver",
            site.function_name,
            site.ip,
        ));
    };
    let Value::Struct { name, .. } = &receiver else {
        return Err(super::err_at(
            VmErrorKind::TypeMismatch,
            "CallMethod receiver must be Struct",
            site.function_name,
            site.ip,
        ));
    };
    let struct_name = name.clone();

    let callee_name = format!("__impl_{}__{}", struct_name, method_name);
    let mut full_args = Vec::with_capacity(argc + 1);
    full_args.push(receiver);
    full_args.append(&mut call_args);
    let ret = super::run_function(
        env.module,
        &callee_name,
        full_args,
        env.host,
        env.reg,
        env.depth + 1,
        env.config,
    )
    .map_err(|e| {
        if e.kind == VmErrorKind::UnknownFunction {
            return super::err_at(
                VmErrorKind::UnknownFunction,
                format!(
                    "Unknown method `{}` on struct `{}`",
                    method_name, struct_name
                ),
                site.function_name,
                site.ip,
            );
        }
        e
    })?;
    stack.push(ret);
    Ok(())
}
