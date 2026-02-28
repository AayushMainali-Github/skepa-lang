use std::time::{SystemTime, UNIX_EPOCH};

use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

#[allow(dead_code)]
pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("datetime", "nowUnix", builtin_datetime_now_unix);
    r.register("datetime", "nowMillis", builtin_datetime_now_millis);
    r.register("datetime", "fromUnix", builtin_datetime_from_unix);
    r.register("datetime", "fromMillis", builtin_datetime_from_millis);
    r.register("datetime", "parseUnix", builtin_datetime_parse_unix);
    r.register("datetime", "year", builtin_datetime_year);
    r.register("datetime", "month", builtin_datetime_month);
    r.register("datetime", "day", builtin_datetime_day);
    r.register("datetime", "hour", builtin_datetime_hour);
    r.register("datetime", "minute", builtin_datetime_minute);
    r.register("datetime", "second", builtin_datetime_second);
}

fn now_duration_since_epoch() -> Result<std::time::Duration, VmError> {
    SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        VmError::new(
            VmErrorKind::HostError,
            "datetime.now* failed: system clock is before Unix epoch",
        )
    })
}

pub(crate) fn builtin_datetime_now_unix(
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

pub(crate) fn builtin_datetime_now_millis(
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

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let mut y = i64::from(year);
    let m = i64::from(month);
    let d = i64::from(day);
    y -= if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
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

fn utc_parts_from_unix_seconds(unix_secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = unix_secs.div_euclid(86_400);
    let sod = unix_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = (sod / 3_600) as u32;
    let minute = ((sod % 3_600) / 60) as u32;
    let second = (sod % 60) as u32;
    (year, month, day, hour, minute, second)
}

pub(crate) fn builtin_datetime_from_unix(
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
    Ok(Value::String(format_utc_from_unix_seconds(ts).into()))
}

pub(crate) fn builtin_datetime_from_millis(
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
    Ok(Value::String(format!("{prefix}.{millis:03}Z").into()))
}

fn parse_u32_field(s: &str, name: &str) -> Result<u32, VmError> {
    s.parse::<u32>().map_err(|_| {
        VmError::new(
            VmErrorKind::TypeMismatch,
            format!("datetime.parseUnix invalid {name}"),
        )
    })
}

fn parse_i32_field(s: &str, name: &str) -> Result<i32, VmError> {
    s.parse::<i32>().map_err(|_| {
        VmError::new(
            VmErrorKind::TypeMismatch,
            format!("datetime.parseUnix invalid {name}"),
        )
    })
}

fn parse_unix_seconds_utc(s: &str) -> Result<i64, VmError> {
    if s.len() != 20 {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "datetime.parseUnix expects format YYYY-MM-DDTHH:MM:SSZ",
        ));
    }
    let b = s.as_bytes();
    if b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "datetime.parseUnix expects format YYYY-MM-DDTHH:MM:SSZ",
        ));
    }

    let year = parse_i32_field(&s[0..4], "year")?;
    let month = parse_u32_field(&s[5..7], "month")?;
    let day = parse_u32_field(&s[8..10], "day")?;
    let hour = parse_u32_field(&s[11..13], "hour")?;
    let minute = parse_u32_field(&s[14..16], "minute")?;
    let second = parse_u32_field(&s[17..19], "second")?;

    if !(1..=12).contains(&month) {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "datetime.parseUnix month out of range",
        ));
    }
    let dim = days_in_month(year, month);
    if day == 0 || day > dim {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "datetime.parseUnix day out of range",
        ));
    }
    if hour > 23 || minute > 59 || second > 59 {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "datetime.parseUnix time out of range",
        ));
    }

    let days = days_from_civil(year, month, day);
    let secs_in_day = i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second);
    days.checked_mul(86_400)
        .and_then(|base| base.checked_add(secs_in_day))
        .ok_or_else(|| {
            VmError::new(
                VmErrorKind::HostError,
                "datetime.parseUnix overflow computing timestamp",
            )
        })
}

pub(crate) fn builtin_datetime_parse_unix(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "datetime.parseUnix expects 1 argument",
        ));
    }
    let Value::String(s) = &args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "datetime.parseUnix expects String argument",
        ));
    };
    Ok(Value::Int(parse_unix_seconds_utc(s)?))
}

fn expect_single_int_arg(args: Vec<Value>, name: &str) -> Result<i64, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            format!("datetime.{name} expects 1 argument"),
        ));
    }
    let Value::Int(ts) = args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            format!("datetime.{name} expects Int argument"),
        ));
    };
    Ok(ts)
}

pub(crate) fn builtin_datetime_year(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    let ts = expect_single_int_arg(args, "year")?;
    let (year, _, _, _, _, _) = utc_parts_from_unix_seconds(ts);
    Ok(Value::Int(i64::from(year)))
}

pub(crate) fn builtin_datetime_month(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    let ts = expect_single_int_arg(args, "month")?;
    let (_, month, _, _, _, _) = utc_parts_from_unix_seconds(ts);
    Ok(Value::Int(i64::from(month)))
}

pub(crate) fn builtin_datetime_day(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    let ts = expect_single_int_arg(args, "day")?;
    let (_, _, day, _, _, _) = utc_parts_from_unix_seconds(ts);
    Ok(Value::Int(i64::from(day)))
}

pub(crate) fn builtin_datetime_hour(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    let ts = expect_single_int_arg(args, "hour")?;
    let (_, _, _, hour, _, _) = utc_parts_from_unix_seconds(ts);
    Ok(Value::Int(i64::from(hour)))
}

pub(crate) fn builtin_datetime_minute(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    let ts = expect_single_int_arg(args, "minute")?;
    let (_, _, _, _, minute, _) = utc_parts_from_unix_seconds(ts);
    Ok(Value::Int(i64::from(minute)))
}

pub(crate) fn builtin_datetime_second(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    let ts = expect_single_int_arg(args, "second")?;
    let (_, _, _, _, _, second) = utc_parts_from_unix_seconds(ts);
    Ok(Value::Int(i64::from(second)))
}
