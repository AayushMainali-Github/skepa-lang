use crate::bytecode::{BytecodeModule, Value};
use crate::vm::{BuiltinHost, BuiltinRegistry, VmConfig, VmError, VmErrorKind};

pub(super) fn call(
    stack: &mut Vec<Value>,
    module: &BytecodeModule,
    callee_name: &str,
    argc: usize,
    host: &mut dyn BuiltinHost,
    reg: &BuiltinRegistry,
    depth: usize,
    config: VmConfig,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    if stack.len() < argc {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on Call",
            function_name,
            ip,
        ));
    }
    let split = stack.len() - argc;
    let call_args = stack.split_off(split);
    let ret = super::run_function(module, callee_name, call_args, host, reg, depth + 1, config)?;
    stack.push(ret);
    Ok(())
}

pub(super) fn call_builtin(
    stack: &mut Vec<Value>,
    host: &mut dyn BuiltinHost,
    reg: &BuiltinRegistry,
    package: &str,
    name: &str,
    argc: usize,
    function_name: &str,
    ip: usize,
) -> Result<(), VmError> {
    if stack.len() < argc {
        return Err(super::err_at(
            VmErrorKind::StackUnderflow,
            "Stack underflow on CallBuiltin",
            function_name,
            ip,
        ));
    }
    let split = stack.len() - argc;
    let call_args = stack.split_off(split);
    let ret = reg.call(host, package, name, call_args)?;
    stack.push(ret);
    Ok(())
}
