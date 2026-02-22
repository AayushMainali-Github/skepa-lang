use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("fs", "exists", builtin_fs_exists);
    r.register("fs", "readText", builtin_fs_read_text);
    r.register("fs", "writeText", builtin_fs_write_text);
    r.register("fs", "appendText", builtin_fs_append_text);
    r.register("fs", "mkdirAll", builtin_fs_mkdir_all);
    r.register("fs", "removeFile", builtin_fs_remove_file);
    r.register("fs", "removeDirAll", builtin_fs_remove_dir_all);
    r.register("fs", "join", builtin_fs_join);
}

fn not_implemented(name: &str) -> Result<Value, VmError> {
    Err(VmError::new(
        VmErrorKind::HostError,
        format!("{name} not implemented yet"),
    ))
}

fn builtin_fs_exists(_host: &mut dyn BuiltinHost, _args: Vec<Value>) -> Result<Value, VmError> {
    if _args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "fs.exists expects 1 argument",
        ));
    }
    let Value::String(path) = &_args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "fs.exists expects String argument",
        ));
    };
    Ok(Value::Bool(std::path::Path::new(path).exists()))
}

fn builtin_fs_read_text(_host: &mut dyn BuiltinHost, _args: Vec<Value>) -> Result<Value, VmError> {
    not_implemented("fs.readText")
}

fn builtin_fs_write_text(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    not_implemented("fs.writeText")
}

fn builtin_fs_append_text(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    not_implemented("fs.appendText")
}

fn builtin_fs_mkdir_all(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    not_implemented("fs.mkdirAll")
}

fn builtin_fs_remove_file(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    not_implemented("fs.removeFile")
}

fn builtin_fs_remove_dir_all(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    not_implemented("fs.removeDirAll")
}

fn builtin_fs_join(_host: &mut dyn BuiltinHost, _args: Vec<Value>) -> Result<Value, VmError> {
    if _args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "fs.join expects 2 arguments",
        ));
    }
    let Value::String(a) = &_args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "fs.join argument 1 expects String",
        ));
    };
    let Value::String(b) = &_args[1] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "fs.join argument 2 expects String",
        ));
    };
    let joined = std::path::PathBuf::from(a).join(b);
    Ok(Value::String(joined.to_string_lossy().into_owned()))
}
