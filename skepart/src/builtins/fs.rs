use crate::{RtHost, RtResult, RtValue};

pub fn exists(host: &mut dyn RtHost, path: &str) -> RtResult<RtValue> {
    Ok(RtValue::Bool(host.fs_exists(path)?))
}

pub fn read_text(host: &mut dyn RtHost, path: &str) -> RtResult<RtValue> {
    match host.fs_read_text(path) {
        Ok(text) => Ok(RtValue::Result(crate::RtResultValue::ok(RtValue::String(
            text,
        )))),
        Err(err) => Ok(RtValue::Result(crate::RtResultValue::err(RtValue::String(
            crate::RtString::from(err.to_string()),
        )))),
    }
}

pub fn write_text(host: &mut dyn RtHost, path: &str, text: &str) -> RtResult<RtValue> {
    host.fs_write_text(path, text)?;
    Ok(RtValue::Unit)
}

pub fn append_text(host: &mut dyn RtHost, path: &str, text: &str) -> RtResult<RtValue> {
    host.fs_append_text(path, text)?;
    Ok(RtValue::Unit)
}

pub fn mkdir_all(host: &mut dyn RtHost, path: &str) -> RtResult<RtValue> {
    host.fs_mkdir_all(path)?;
    Ok(RtValue::Unit)
}

pub fn remove_file(host: &mut dyn RtHost, path: &str) -> RtResult<RtValue> {
    host.fs_remove_file(path)?;
    Ok(RtValue::Unit)
}

pub fn remove_dir_all(host: &mut dyn RtHost, path: &str) -> RtResult<RtValue> {
    host.fs_remove_dir_all(path)?;
    Ok(RtValue::Unit)
}

pub fn join(host: &mut dyn RtHost, left: &str, right: &str) -> RtResult<RtValue> {
    Ok(RtValue::String(host.fs_join(left, right)?))
}
