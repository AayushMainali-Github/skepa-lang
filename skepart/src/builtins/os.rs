use crate::{RtHost, RtResult, RtValue};

pub fn cwd(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::String(host.os_cwd()?))
}

pub fn platform(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::String(host.os_platform()?))
}

pub fn sleep(host: &mut dyn RtHost, millis: i64) -> RtResult<RtValue> {
    host.os_sleep(millis)?;
    Ok(RtValue::Unit)
}

pub fn exec_shell(host: &mut dyn RtHost, command: &str) -> RtResult<RtValue> {
    Ok(RtValue::Int(host.os_exec_shell(command)?))
}

pub fn exec_shell_out(host: &mut dyn RtHost, command: &str) -> RtResult<RtValue> {
    Ok(RtValue::String(host.os_exec_shell_out(command)?))
}
