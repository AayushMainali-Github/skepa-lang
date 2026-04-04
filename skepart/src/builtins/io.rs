use crate::{RtHost, RtResult, RtValue};

pub fn print(host: &mut dyn RtHost, value: &RtValue) -> RtResult<()> {
    host.io_print(&display_value(value))
}

pub fn println(host: &mut dyn RtHost, value: &RtValue) -> RtResult<()> {
    host.io_println(&display_value(value))
}

pub fn read_line(host: &mut dyn RtHost) -> RtResult<RtValue> {
    Ok(RtValue::String(host.io_read_line()?))
}

pub fn format(args: &[RtValue]) -> RtResult<RtValue> {
    if args.is_empty() {
        return Err(crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            "io.format expects at least 1 argument",
        ));
    }
    let fmt = args[0].expect_string()?;
    Ok(RtValue::String(crate::RtString::from(apply_format(
        fmt.as_str(),
        &args[1..],
    )?)))
}

pub fn printf(host: &mut dyn RtHost, args: &[RtValue]) -> RtResult<RtValue> {
    let RtValue::String(text) = format(args)? else {
        unreachable!()
    };
    host.io_print(text.as_str())?;
    Ok(RtValue::Unit)
}

fn display_value(value: &RtValue) -> String {
    match value {
        RtValue::Int(value) => value.to_string(),
        RtValue::Float(value) => value.to_string(),
        RtValue::Bool(value) => value.to_string(),
        RtValue::String(value) => value.as_str().to_owned(),
        RtValue::Bytes(value) => format!("[bytes len={}]", value.len()),
        RtValue::Option(value) => match &value.0 {
            Some(inner) => format!("Some({})", display_value(inner)),
            None => "None".to_owned(),
        },
        RtValue::Array(_) => "[array]".to_owned(),
        RtValue::Vec(_) => "[vec]".to_owned(),
        RtValue::Map(_) => "[map]".to_owned(),
        RtValue::Function(_) => "[function]".to_owned(),
        RtValue::Handle(value) => format!("[handle {:?}#{}]", value.kind, value.id),
        RtValue::Struct(value) => format!("[struct {}]", value.layout.name),
        RtValue::Unit => String::new(),
    }
}

fn apply_format(fmt: &str, args: &[RtValue]) -> RtResult<String> {
    let mut out = String::new();
    let mut chars = fmt.chars().peekable();
    let mut idx = 0usize;

    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }
        let Some(spec) = chars.next() else {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "io.format format string ends with `%`",
            ));
        };
        if spec == '%' {
            out.push('%');
            continue;
        }
        let Some(value) = args.get(idx) else {
            return Err(crate::RtError::new(
                crate::RtErrorKind::InvalidArgument,
                "io.format received too few arguments for format string",
            ));
        };
        idx += 1;
        match spec {
            'd' => out.push_str(&value.expect_int()?.to_string()),
            'f' => out.push_str(&value.expect_float()?.to_string()),
            'b' => out.push_str(&value.expect_bool()?.to_string()),
            's' => out.push_str(value.expect_string()?.as_str()),
            _ => {
                return Err(crate::RtError::new(
                    crate::RtErrorKind::InvalidArgument,
                    format!("Unsupported format specifier `%{spec}`"),
                ));
            }
        }
    }

    if idx != args.len() {
        return Err(crate::RtError::new(
            crate::RtErrorKind::InvalidArgument,
            "io.format received too many arguments for format string",
        ));
    }

    Ok(out)
}
