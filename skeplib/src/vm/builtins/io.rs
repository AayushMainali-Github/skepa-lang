use crate::bytecode::Value;

use super::{BuiltinHost, BuiltinRegistry, VmError, VmErrorKind};

pub(super) fn register(r: &mut BuiltinRegistry) {
    r.register("io", "print", builtin_io_print);
    r.register("io", "println", builtin_io_println);
    r.register("io", "printInt", builtin_io_print_int);
    r.register("io", "printFloat", builtin_io_print_float);
    r.register("io", "printBool", builtin_io_print_bool);
    r.register("io", "printString", builtin_io_print_string);
    r.register("io", "format", builtin_io_format);
    r.register("io", "printf", builtin_io_printf);
    r.register("io", "readLine", builtin_io_readline);
}

fn builtin_io_print(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "io.print expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => host.write(s, false)?,
        _ => {
            return Err(VmError::new(
                VmErrorKind::TypeMismatch,
                "io.print expects String argument",
            ));
        }
    }
    Ok(Value::Unit)
}

fn builtin_io_println(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "io.println expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => host.write(s, true)?,
        _ => {
            return Err(VmError::new(
                VmErrorKind::TypeMismatch,
                "io.println expects String argument",
            ));
        }
    }
    Ok(Value::Unit)
}

fn builtin_io_readline(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if !args.is_empty() {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "io.readLine expects 0 arguments",
        ));
    }
    Ok(Value::String(host.read_line()?))
}

fn builtin_io_print_int(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "io.printInt expects 1 argument",
        ));
    }
    match &args[0] {
        Value::Int(v) => host.write(&v.to_string(), true)?,
        _ => {
            return Err(VmError::new(
                VmErrorKind::TypeMismatch,
                "io.printInt expects Int argument",
            ));
        }
    }
    Ok(Value::Unit)
}

fn builtin_io_print_float(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "io.printFloat expects 1 argument",
        ));
    }
    match &args[0] {
        Value::Float(v) => host.write(&v.to_string(), true)?,
        _ => {
            return Err(VmError::new(
                VmErrorKind::TypeMismatch,
                "io.printFloat expects Float argument",
            ));
        }
    }
    Ok(Value::Unit)
}

fn builtin_io_print_bool(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "io.printBool expects 1 argument",
        ));
    }
    match &args[0] {
        Value::Bool(v) => host.write(&v.to_string(), true)?,
        _ => {
            return Err(VmError::new(
                VmErrorKind::TypeMismatch,
                "io.printBool expects Bool argument",
            ));
        }
    }
    Ok(Value::Unit)
}

fn builtin_io_print_string(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "io.printString expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => host.write(s, true)?,
        _ => {
            return Err(VmError::new(
                VmErrorKind::TypeMismatch,
                "io.printString expects String argument",
            ));
        }
    }
    Ok(Value::Unit)
}

fn parse_format_specifiers(fmt: &str) -> Result<Vec<char>, VmError> {
    let chars: Vec<char> = fmt.chars().collect();
    let mut specs = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        if i + 1 >= chars.len() {
            return Err(VmError::new(
                VmErrorKind::TypeMismatch,
                "format string ends with `%`",
            ));
        }
        let spec = chars[i + 1];
        match spec {
            '%' => {}
            'd' | 'f' | 's' | 'b' => specs.push(spec),
            other => {
                return Err(VmError::new(
                    VmErrorKind::TypeMismatch,
                    format!("unsupported format specifier `%{other}`"),
                ));
            }
        }
        i += 2;
    }
    Ok(specs)
}

fn render_format(fmt: &str, values: &[Value]) -> Result<String, VmError> {
    let specs = parse_format_specifiers(fmt)?;
    if specs.len() != values.len() {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            format!(
                "format expects {} value argument(s), got {}",
                specs.len(),
                values.len()
            ),
        ));
    }
    let mut out = String::new();
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0usize;
    let mut arg_idx = 0usize;
    while i < chars.len() {
        if chars[i] != '%' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let spec = chars[i + 1];
        match spec {
            '%' => out.push('%'),
            'd' => match &values[arg_idx] {
                Value::Int(v) => out.push_str(&v.to_string()),
                _ => {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        format!("format argument {} must be Int for `%d`", arg_idx + 2),
                    ));
                }
            },
            'f' => match &values[arg_idx] {
                Value::Float(v) => out.push_str(&v.to_string()),
                _ => {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        format!("format argument {} must be Float for `%f`", arg_idx + 2),
                    ));
                }
            },
            's' => match &values[arg_idx] {
                Value::String(v) => out.push_str(v),
                _ => {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        format!("format argument {} must be String for `%s`", arg_idx + 2),
                    ));
                }
            },
            'b' => match &values[arg_idx] {
                Value::Bool(v) => out.push_str(&v.to_string()),
                _ => {
                    return Err(VmError::new(
                        VmErrorKind::TypeMismatch,
                        format!("format argument {} must be Bool for `%b`", arg_idx + 2),
                    ));
                }
            },
            _ => unreachable!(),
        }
        if spec != '%' {
            arg_idx += 1;
        }
        i += 2;
    }
    Ok(out)
}

fn builtin_io_format(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "io.format expects at least 1 argument",
        ));
    }
    let Value::String(fmt) = &args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "io.format argument 1 expects String",
        ));
    };
    let rendered = render_format(fmt, &args[1..])?;
    Ok(Value::String(rendered))
}

fn builtin_io_printf(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "io.printf expects at least 1 argument",
        ));
    }
    let Value::String(fmt) = &args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "io.printf argument 1 expects String",
        ));
    };
    let rendered = render_format(fmt, &args[1..])?;
    host.write(&rendered, false)?;
    Ok(Value::Unit)
}
