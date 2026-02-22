use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("os", "cwd", builtin_os_cwd);
    r.register("os", "platform", builtin_os_platform);
    r.register("os", "sleep", builtin_os_sleep);
    r.register("os", "execShell", builtin_os_exec_shell);
    r.register("os", "execShellOut", builtin_os_exec_shell_out);
}

fn not_implemented(name: &str) -> Result<Value, VmError> {
    Err(VmError::new(
        VmErrorKind::HostError,
        format!("{name} not implemented yet"),
    ))
}

fn builtin_os_cwd(_host: &mut dyn BuiltinHost, _args: Vec<Value>) -> Result<Value, VmError> {
    not_implemented("os.cwd")
}

fn builtin_os_platform(_host: &mut dyn BuiltinHost, _args: Vec<Value>) -> Result<Value, VmError> {
    not_implemented("os.platform")
}

fn builtin_os_sleep(_host: &mut dyn BuiltinHost, _args: Vec<Value>) -> Result<Value, VmError> {
    not_implemented("os.sleep")
}

fn builtin_os_exec_shell(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    not_implemented("os.execShell")
}

fn builtin_os_exec_shell_out(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    not_implemented("os.execShellOut")
}
