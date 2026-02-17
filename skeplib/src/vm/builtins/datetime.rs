use std::time::{SystemTime, UNIX_EPOCH};

use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("datetime", "nowUnix", builtin_datetime_now_unix);
    r.register("datetime", "nowMillis", builtin_datetime_now_millis);
    r.register("datetime", "fromUnix", builtin_datetime_from_unix);
    r.register("datetime", "fromMillis", builtin_datetime_from_millis);
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

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    (y as i32, m as u32, d as u32)
}

fn format_utc_from_unix_seconds(unix_secs: i64) -> String {
    let days = unix_secs.div_euclid(86_400);
    let sod = unix_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = sod / 3_600;
    let minute = (sod % 3_600) / 60;
    let second = sod % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn builtin_datetime_from_unix(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "datetime.fromUnix expects 1 argument",
        ));
    }
    let Value::Int(ts) = args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "datetime.fromUnix expects Int argument",
        ));
    };
    Ok(Value::String(format_utc_from_unix_seconds(ts)))
}

fn builtin_datetime_from_millis(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "datetime.fromMillis expects 1 argument",
        ));
    }
    let Value::Int(ms) = args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "datetime.fromMillis expects Int argument",
        ));
    };
    let secs = ms.div_euclid(1_000);
    let millis = ms.rem_euclid(1_000);
    let base = format_utc_from_unix_seconds(secs);
    let prefix = base.strip_suffix('Z').unwrap_or(&base);
    Ok(Value::String(format!("{prefix}.{millis:03}Z")))
}
