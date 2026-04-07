use crate::{RtHost, RtOption, RtResult, RtResultValue, RtValue};

pub fn platform(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::String(host.os_platform()?))
}

pub fn arch(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::String(host.os_arch()?))
}

pub fn arg(host: &mut dyn RtHost, index: i64) -> RtResult<RtValue> {
    Ok(RtValue::Option(match host.os_arg(index) {
        Ok(value) => RtOption::some(RtValue::String(value)),
        Err(_) => RtOption::none(),
    }))
}

pub fn env_has(host: &mut dyn RtHost, name: &str) -> RtResult<RtValue> {
    Ok(RtValue::Bool(host.os_env_has(name)?))
}

pub fn env_get(host: &mut dyn RtHost, name: &str) -> RtResult<RtValue> {
    Ok(RtValue::Option(match host.os_env_get(name)? {
        Some(value) => RtOption::some(RtValue::String(value)),
        None => RtOption::none(),
    }))
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
    Ok(RtValue::Result(match host.os_exec(program, args) {
        Ok(code) => RtResultValue::ok(RtValue::Int(code)),
        Err(err) => RtResultValue::err(RtValue::String(err.to_string().into())),
    }))
}

pub fn exec_out(host: &mut dyn RtHost, program: &str, args: &[String]) -> RtResult<RtValue> {
    Ok(RtValue::Result(match host.os_exec_out(program, args) {
        Ok(output) => RtResultValue::ok(RtValue::String(output)),
        Err(err) => RtResultValue::err(RtValue::String(err.to_string().into())),
    }))
}
