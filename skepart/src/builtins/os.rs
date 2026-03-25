use crate::{RtHost, RtResult, RtValue};

pub fn platform(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::String(host.os_platform()?))
}

pub fn arch(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::String(host.os_arch()?))
}

pub fn arg(host: &mut dyn RtHost, index: i64) -> RtResult<RtValue> {
    Ok(RtValue::String(host.os_arg(index)?))
}

pub fn env_has(host: &mut dyn RtHost, name: &str) -> RtResult<RtValue> {
    Ok(RtValue::Bool(host.os_env_has(name)?))
}

pub fn env_get(host: &mut dyn RtHost, name: &str) -> RtResult<RtValue> {
    Ok(RtValue::String(host.os_env_get(name)?))
}

pub fn env_set(host: &mut dyn RtHost, name: &str, value: &str) -> RtResult<RtValue> {
    host.os_env_set(name, value)?;
    Ok(RtValue::Unit)
}

pub fn env_remove(host: &mut dyn RtHost, name: &str) -> RtResult<RtValue> {
    host.os_env_remove(name)?;
    Ok(RtValue::Unit)
}

pub fn sleep(host: &mut dyn RtHost, millis: i64) -> RtResult<RtValue> {
    host.os_sleep(millis)?;
    Ok(RtValue::Unit)
}

pub fn exit(host: &mut dyn RtHost, code: i64) -> RtResult<RtValue> {
    host.os_exit(code)?;
    Ok(RtValue::Unit)
}

pub fn exec(host: &mut dyn RtHost, program: &str, args: &[String]) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.os_exec(program, args)?))
}

pub fn exec_out(host: &mut dyn RtHost, program: &str, args: &[String]) -> RtResult<RtValue> {
    Ok(RtValue::String(host.os_exec_out(program, args)?))
}
