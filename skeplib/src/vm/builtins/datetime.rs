use std::time::{SystemTime, UNIX_EPOCH};

use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("datetime", "nowUnix", builtin_datetime_now_unix);
    r.register("datetime", "nowMillis", builtin_datetime_now_millis);
}

fn now_duration_since_epoch() -> Result<std::time::Duration, VmError> {
    SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        VmError::new(
            VmErrorKind::HostError,
            "datetime.now* failed: system clock is before Unix epoch",
        )
    })
}

fn builtin_datetime_now_unix(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if !args.is_empty() {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "datetime.nowUnix expects 0 arguments",
        ));
    }
    let secs = now_duration_since_epoch()?.as_secs();
    let secs = i64::try_from(secs).map_err(|_| {
        VmError::new(
            VmErrorKind::HostError,
            "datetime.nowUnix overflow converting timestamp",
        )
    })?;
    Ok(Value::Int(secs))
}

fn builtin_datetime_now_millis(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if !args.is_empty() {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "datetime.nowMillis expects 0 arguments",
        ));
    }
    let duration = now_duration_since_epoch()?;
    let millis = duration
        .as_secs()
        .checked_mul(1000)
        .and_then(|v| v.checked_add(u64::from(duration.subsec_millis())))
        .ok_or_else(|| {
            VmError::new(
                VmErrorKind::HostError,
                "datetime.nowMillis overflow computing timestamp",
            )
        })?;
    let millis = i64::try_from(millis).map_err(|_| {
        VmError::new(
            VmErrorKind::HostError,
            "datetime.nowMillis overflow converting timestamp",
        )
    })?;
    Ok(Value::Int(millis))
}

