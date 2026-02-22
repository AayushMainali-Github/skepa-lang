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
    if !_args.is_empty() {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "os.cwd expects 0 arguments",
        ));
    }
    let cwd = std::env::current_dir().map_err(|e| {
        VmError::new(VmErrorKind::HostError, format!("os.cwd failed: {e}"))
    })?;
    Ok(Value::String(cwd.to_string_lossy().into_owned()))
}

fn builtin_os_platform(_host: &mut dyn BuiltinHost, _args: Vec<Value>) -> Result<Value, VmError> {
    if !_args.is_empty() {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "os.platform expects 0 arguments",
        ));
    }
    let name = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    Ok(Value::String(name.to_string()))
}

fn builtin_os_sleep(_host: &mut dyn BuiltinHost, _args: Vec<Value>) -> Result<Value, VmError> {
    if _args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "os.sleep expects 1 argument",
        ));
    }
    let Value::Int(ms) = _args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "os.sleep expects Int argument",
        ));
    };
    if ms < 0 {
        return Err(VmError::new(
            VmErrorKind::HostError,
            "os.sleep expects non-negative milliseconds",
        ));
    }
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    Ok(Value::Unit)
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
