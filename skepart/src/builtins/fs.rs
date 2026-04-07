use crate::{RtHost, RtResult, RtResultValue, RtString, RtValue};

pub fn exists(host: &mut dyn RtHost, path: &str) -> RtResult<RtValue> {
    match host.fs_exists(path) {
        Ok(value) => Ok(RtValue::Result(RtResultValue::ok(RtValue::Bool(value)))),
        Err(err) => Ok(RtValue::Result(RtResultValue::err(RtValue::String(
            RtString::from(err.to_string()),
        )))),
    }
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
    match host.fs_write_text(path, text) {
        Ok(()) => Ok(RtValue::Result(crate::RtResultValue::ok(RtValue::Unit))),
        Err(err) => Ok(RtValue::Result(crate::RtResultValue::err(RtValue::String(
            crate::RtString::from(err.to_string()),
        )))),
    }
}

pub fn append_text(host: &mut dyn RtHost, path: &str, text: &str) -> RtResult<RtValue> {
    match host.fs_append_text(path, text) {
        Ok(()) => Ok(RtValue::Result(crate::RtResultValue::ok(RtValue::Unit))),
        Err(err) => Ok(RtValue::Result(crate::RtResultValue::err(RtValue::String(
            crate::RtString::from(err.to_string()),
        )))),
    }
}

pub fn mkdir_all(host: &mut dyn RtHost, path: &str) -> RtResult<RtValue> {
    match host.fs_mkdir_all(path) {
        Ok(()) => Ok(RtValue::Result(crate::RtResultValue::ok(RtValue::Unit))),
        Err(err) => Ok(RtValue::Result(crate::RtResultValue::err(RtValue::String(
            crate::RtString::from(err.to_string()),
        )))),
    }
}

pub fn remove_file(host: &mut dyn RtHost, path: &str) -> RtResult<RtValue> {
    match host.fs_remove_file(path) {
        Ok(()) => Ok(RtValue::Result(crate::RtResultValue::ok(RtValue::Unit))),
        Err(err) => Ok(RtValue::Result(crate::RtResultValue::err(RtValue::String(
            crate::RtString::from(err.to_string()),
        )))),
    }
}

pub fn remove_dir_all(host: &mut dyn RtHost, path: &str) -> RtResult<RtValue> {
    match host.fs_remove_dir_all(path) {
        Ok(()) => Ok(RtValue::Result(crate::RtResultValue::ok(RtValue::Unit))),
        Err(err) => Ok(RtValue::Result(crate::RtResultValue::err(RtValue::String(
            crate::RtString::from(err.to_string()),
        )))),
    }
}

pub fn join(host: &mut dyn RtHost, left: &str, right: &str) -> RtResult<RtValue> {
    Ok(RtValue::String(host.fs_join(left, right)?))
}
