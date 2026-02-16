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
