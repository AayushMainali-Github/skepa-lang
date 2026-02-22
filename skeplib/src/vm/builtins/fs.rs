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
    let exists = std::path::Path::new(path)
        .try_exists()
        .map_err(|e| VmError::new(VmErrorKind::HostError, format!("fs.exists failed: {e}")))?;
    Ok(Value::Bool(exists))
}

fn builtin_fs_read_text(_host: &mut dyn BuiltinHost, _args: Vec<Value>) -> Result<Value, VmError> {
    if _args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "fs.readText expects 1 argument",
        ));
    }
    let Value::String(path) = &_args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "fs.readText expects String argument",
        ));
    };
    let text = std::fs::read_to_string(path)
        .map_err(|e| VmError::new(VmErrorKind::HostError, format!("fs.readText failed: {e}")))?;
    Ok(Value::String(text))
}

fn builtin_fs_write_text(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    if _args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "fs.writeText expects 2 arguments",
        ));
    }
    let Value::String(path) = &_args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "fs.writeText argument 1 expects String",
        ));
    };
    let Value::String(data) = &_args[1] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "fs.writeText argument 2 expects String",
        ));
    };
    std::fs::write(path, data)
        .map_err(|e| VmError::new(VmErrorKind::HostError, format!("fs.writeText failed: {e}")))?;
    Ok(Value::Unit)
}

fn builtin_fs_append_text(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    if _args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "fs.appendText expects 2 arguments",
        ));
    }
    let Value::String(path) = &_args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "fs.appendText argument 1 expects String",
        ));
    };
    let Value::String(data) = &_args[1] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "fs.appendText argument 2 expects String",
        ));
    };
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| {
            VmError::new(VmErrorKind::HostError, format!("fs.appendText failed: {e}"))
        })?;
    use std::io::Write as _;
    f.write_all(data.as_bytes())
        .map_err(|e| VmError::new(VmErrorKind::HostError, format!("fs.appendText failed: {e}")))?;
    Ok(Value::Unit)
}

fn builtin_fs_mkdir_all(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    if _args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "fs.mkdirAll expects 1 argument",
        ));
    }
    let Value::String(path) = &_args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "fs.mkdirAll expects String argument",
        ));
    };
    std::fs::create_dir_all(path)
        .map_err(|e| VmError::new(VmErrorKind::HostError, format!("fs.mkdirAll failed: {e}")))?;
    Ok(Value::Unit)
}

fn builtin_fs_remove_file(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    if _args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "fs.removeFile expects 1 argument",
        ));
    }
    let Value::String(path) = &_args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "fs.removeFile expects String argument",
        ));
    };
    std::fs::remove_file(path)
        .map_err(|e| VmError::new(VmErrorKind::HostError, format!("fs.removeFile failed: {e}")))?;
    Ok(Value::Unit)
}

fn builtin_fs_remove_dir_all(
    _host: &mut dyn BuiltinHost,
    _args: Vec<Value>,
) -> Result<Value, VmError> {
    if _args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "fs.removeDirAll expects 1 argument",
        ));
    }
    let Value::String(path) = &_args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "fs.removeDirAll expects String argument",
        ));
    };
    std::fs::remove_dir_all(path).map_err(|e| {
        VmError::new(VmErrorKind::HostError, format!("fs.removeDirAll failed: {e}"))
    })?;
    Ok(Value::Unit)
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
