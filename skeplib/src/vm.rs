use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::io::{self, Write};

use crate::bytecode::{BytecodeModule, FunctionChunk, Instr, Value};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vm;
const DEFAULT_MAX_CALL_DEPTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmConfig {
    pub max_call_depth: usize,
    pub trace: bool,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
            trace: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmErrorKind {
    UnknownFunction,
    ArityMismatch,
    StackUnderflow,
    TypeMismatch,
    InvalidLocal,
    DivisionByZero,
    UnknownBuiltin,
    HostError,
    StackOverflow,
    IndexOutOfBounds,
}

impl VmErrorKind {
    pub fn code(self) -> &'static str {
        match self {
            VmErrorKind::UnknownFunction => "E-VM-UNKNOWN-FUNCTION",
            VmErrorKind::ArityMismatch => "E-VM-ARITY",
            VmErrorKind::StackUnderflow => "E-VM-STACK-UNDERFLOW",
            VmErrorKind::TypeMismatch => "E-VM-TYPE",
            VmErrorKind::InvalidLocal => "E-VM-INVALID-LOCAL",
            VmErrorKind::DivisionByZero => "E-VM-DIV-ZERO",
            VmErrorKind::UnknownBuiltin => "E-VM-UNKNOWN-BUILTIN",
            VmErrorKind::HostError => "E-VM-HOST",
            VmErrorKind::StackOverflow => "E-VM-STACK-OVERFLOW",
            VmErrorKind::IndexOutOfBounds => "E-VM-INDEX-OOB",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VmError {
    pub kind: VmErrorKind,
    pub message: String,
}

impl VmError {
    fn new(kind: VmErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub trait BuiltinHost {
    fn write(&mut self, s: &str, newline: bool) -> Result<(), VmError>;
    fn read_line(&mut self) -> Result<String, VmError>;
}

pub struct StdIoHost;

impl BuiltinHost for StdIoHost {
    fn write(&mut self, s: &str, newline: bool) -> Result<(), VmError> {
        if newline {
            println!("{s}");
        } else {
            print!("{s}");
            io::stdout()
                .flush()
                .map_err(|e| VmError::new(VmErrorKind::HostError, e.to_string()))?;
        }
        Ok(())
    }

    fn read_line(&mut self) -> Result<String, VmError> {
        let mut buf = String::new();
        io::stdin()
            .read_line(&mut buf)
            .map_err(|e| VmError::new(VmErrorKind::HostError, e.to_string()))?;
        while buf.ends_with('\n') || buf.ends_with('\r') {
            buf.pop();
        }
        Ok(buf)
    }
}

#[derive(Default)]
pub struct TestHost {
    pub output: String,
    pub input: VecDeque<String>,
}

impl BuiltinHost for TestHost {
    fn write(&mut self, s: &str, newline: bool) -> Result<(), VmError> {
        self.output.push_str(s);
        if newline {
            self.output.push('\n');
        }
        Ok(())
    }

    fn read_line(&mut self) -> Result<String, VmError> {
        Ok(self.input.pop_front().unwrap_or_default())
    }
}

pub type BuiltinHandler = fn(&mut dyn BuiltinHost, Vec<Value>) -> Result<Value, VmError>;

#[derive(Default)]
pub struct BuiltinRegistry {
    handlers: HashMap<String, BuiltinHandler>,
}

impl BuiltinRegistry {
    pub fn with_defaults() -> Self {
        let mut r = Self::default();
        r.register("io", "print", builtin_io_print);
        r.register("io", "println", builtin_io_println);
        r.register("io", "printInt", builtin_io_print_int);
        r.register("io", "printFloat", builtin_io_print_float);
        r.register("io", "printBool", builtin_io_print_bool);
        r.register("io", "printString", builtin_io_print_string);
        r.register("io", "format", builtin_io_format);
        r.register("io", "printf", builtin_io_printf);
        r.register("io", "readLine", builtin_io_readline);
        r.register("str", "len", builtin_str_len);
        r.register("str", "contains", builtin_str_contains);
        r.register("str", "startsWith", builtin_str_starts_with);
        r.register("str", "endsWith", builtin_str_ends_with);
        r.register("str", "trim", builtin_str_trim);
        r.register("str", "toLower", builtin_str_to_lower);
        r.register("str", "toUpper", builtin_str_to_upper);
        r.register("str", "indexOf", builtin_str_index_of);
        r.register("str", "slice", builtin_str_slice);
        r.register("str", "isEmpty", builtin_str_is_empty);
        r.register("arr", "len", builtin_arr_len);
        r.register("arr", "isEmpty", builtin_arr_is_empty);
        r.register("arr", "contains", builtin_arr_contains);
        r.register("arr", "indexOf", builtin_arr_index_of);
        r.register("arr", "sum", builtin_arr_sum);
        r
    }

    fn key(package: &str, name: &str) -> String {
        format!("{package}.{name}")
    }

    pub fn register(&mut self, package: &str, name: &str, handler: BuiltinHandler) {
        self.handlers.insert(Self::key(package, name), handler);
    }

    fn call(
        &self,
        host: &mut dyn BuiltinHost,
        package: &str,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, VmError> {
        let key = Self::key(package, name);
        let Some(handler) = self.handlers.get(&key).copied() else {
            return Err(VmError::new(
                VmErrorKind::UnknownBuiltin,
                format!("Unknown builtin `{key}`"),
            ));
        };
        handler(host, args)
    }
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

fn builtin_str_len(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.len expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.len expects String argument",
        )),
    }
}

fn builtin_str_contains(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.contains expects 2 arguments",
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(haystack), Value::String(needle)) => {
            Ok(Value::Bool(haystack.contains(needle)))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.contains expects String, String arguments",
        )),
    }
}

fn builtin_str_starts_with(
    _host: &mut dyn BuiltinHost,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.startsWith expects 2 arguments",
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(prefix)) => Ok(Value::Bool(s.starts_with(prefix))),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.startsWith expects String, String arguments",
        )),
    }
}

fn builtin_str_ends_with(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.endsWith expects 2 arguments",
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(suffix)) => Ok(Value::Bool(s.ends_with(suffix))),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.endsWith expects String, String arguments",
        )),
    }
}

fn builtin_str_trim(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.trim expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.trim().to_string())),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.trim expects String argument",
        )),
    }
}

fn builtin_str_to_lower(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.toLower expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_lowercase())),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.toLower expects String argument",
        )),
    }
}

fn builtin_str_to_upper(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.toUpper expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_uppercase())),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.toUpper expects String argument",
        )),
    }
}

fn builtin_str_index_of(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.indexOf expects 2 arguments",
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(needle)) => match s.find(needle) {
            Some(byte_idx) => Ok(Value::Int(s[..byte_idx].chars().count() as i64)),
            None => Ok(Value::Int(-1)),
        },
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.indexOf expects String, String arguments",
        )),
    }
}

fn builtin_str_slice(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 3 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.slice expects 3 arguments",
        ));
    }
    let (Value::String(s), Value::Int(start), Value::Int(end)) = (&args[0], &args[1], &args[2])
    else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.slice expects String, Int, Int arguments",
        ));
    };

    let len = s.chars().count() as i64;
    if *start < 0 || *end < 0 || *start > *end || *end > len {
        return Err(VmError::new(
            VmErrorKind::IndexOutOfBounds,
            format!(
                "str.slice bounds out of range: start={}, end={}, len={len}",
                start, end
            ),
        ));
    }

    let out: String = s
        .chars()
        .skip(*start as usize)
        .take((*end - *start) as usize)
        .collect();
    Ok(Value::String(out))
}

fn builtin_str_is_empty(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "str.isEmpty expects 1 argument",
        ));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Bool(s.is_empty())),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "str.isEmpty expects String argument",
        )),
    }
}

fn builtin_arr_len(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.len expects 1 argument",
        ));
    }
    match &args[0] {
        Value::Array(items) => Ok(Value::Int(items.len() as i64)),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.len expects Array argument",
        )),
    }
}

fn builtin_arr_is_empty(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.isEmpty expects 1 argument",
        ));
    }
    match &args[0] {
        Value::Array(items) => Ok(Value::Bool(items.is_empty())),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.isEmpty expects Array argument",
        )),
    }
}

fn builtin_arr_contains(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.contains expects 2 arguments",
        ));
    }
    match &args[0] {
        Value::Array(items) => Ok(Value::Bool(items.contains(&args[1]))),
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.contains expects Array as first argument",
        )),
    }
}

fn builtin_arr_index_of(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.indexOf expects 2 arguments",
        ));
    }
    match &args[0] {
        Value::Array(items) => {
            let idx = items
                .iter()
                .position(|v| v == &args[1])
                .map(|i| i as i64)
                .unwrap_or(-1);
            Ok(Value::Int(idx))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.indexOf expects Array as first argument",
        )),
    }
}

fn arr_sum_step(lhs: Value, rhs: Value) -> Result<Value, VmError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),
        (Value::Array(mut a), Value::Array(b)) => {
            a.extend(b);
            Ok(Value::Array(a))
        }
        _ => Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.sum supports Int, Float, String, or Array element types",
        )),
    }
}

fn builtin_arr_sum(_host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::new(
            VmErrorKind::ArityMismatch,
            "arr.sum expects 1 argument",
        ));
    }
    let Value::Array(items) = &args[0] else {
        return Err(VmError::new(
            VmErrorKind::TypeMismatch,
            "arr.sum expects Array argument",
        ));
    };
    if items.is_empty() {
        return Ok(Value::Int(0));
    }
    let mut acc = items[0].clone();
    for v in items.iter().skip(1) {
        acc = arr_sum_step(acc, v.clone())?;
    }
    Ok(acc)
}

impl Vm {
    pub fn run_module_main(module: &BytecodeModule) -> Result<Value, VmError> {
        Self::run_module_main_with_config(module, VmConfig::default())
    }

    pub fn run_module_main_with_config(
        module: &BytecodeModule,
        config: VmConfig,
    ) -> Result<Value, VmError> {
        let mut host = StdIoHost;
        let reg = BuiltinRegistry::with_defaults();
        Self::run_function(module, "main", Vec::new(), &mut host, &reg, 0, config)
    }

    pub fn run_module_main_with_host(
        module: &BytecodeModule,
        host: &mut dyn BuiltinHost,
    ) -> Result<Value, VmError> {
        let reg = BuiltinRegistry::with_defaults();
        Self::run_function(
            module,
            "main",
            Vec::new(),
            host,
            &reg,
            0,
            VmConfig::default(),
        )
    }

    pub fn run_module_main_with_registry(
        module: &BytecodeModule,
        host: &mut dyn BuiltinHost,
        reg: &BuiltinRegistry,
    ) -> Result<Value, VmError> {
        Self::run_function(
            module,
            "main",
            Vec::new(),
            host,
            reg,
            0,
            VmConfig::default(),
        )
    }

    pub fn run_main(chunk: &FunctionChunk) -> Result<Value, VmError> {
        let module = BytecodeModule {
            functions: vec![(chunk.name.clone(), chunk.clone())]
                .into_iter()
                .collect(),
        };
        Self::run_module_main(&module)
    }

    fn run_function(
        module: &BytecodeModule,
        function_name: &str,
        args: Vec<Value>,
        host: &mut dyn BuiltinHost,
        reg: &BuiltinRegistry,
        depth: usize,
        config: VmConfig,
    ) -> Result<Value, VmError> {
        if depth >= config.max_call_depth {
            return Err(VmError::new(
                VmErrorKind::StackOverflow,
                format!("Call stack limit exceeded ({})", config.max_call_depth),
            ));
        }
        let Some(chunk) = module.functions.get(function_name) else {
            return Err(VmError::new(
                VmErrorKind::UnknownFunction,
                format!("Unknown function `{function_name}`"),
            ));
        };
        if args.len() != chunk.param_count {
            return Err(VmError::new(
                VmErrorKind::ArityMismatch,
                format!(
                    "Function `{}` arity mismatch: expected {}, got {}",
                    function_name,
                    chunk.param_count,
                    args.len()
                ),
            ));
        }

        let mut stack: Vec<Value> = Vec::new();
        let mut locals: Vec<Value> = vec![Value::Unit; chunk.locals_count.max(1)];
        for (i, arg) in args.into_iter().enumerate() {
            if i < locals.len() {
                locals[i] = arg;
            }
        }

        let mut ip = 0usize;
        while ip < chunk.code.len() {
            if config.trace {
                eprintln!("[trace] {}@{} {:?}", function_name, ip, chunk.code[ip]);
            }
            match &chunk.code[ip] {
                Instr::LoadConst(v) => stack.push(v.clone()),
                Instr::LoadLocal(slot) => {
                    let Some(v) = locals.get(*slot).cloned() else {
                        return Err(Self::err_at(
                            VmErrorKind::InvalidLocal,
                            format!("Invalid local slot {slot}"),
                            function_name,
                            ip,
                        ));
                    };
                    stack.push(v);
                }
                Instr::StoreLocal(slot) => {
                    let Some(v) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "Stack underflow on StoreLocal",
                            function_name,
                            ip,
                        ));
                    };
                    if *slot >= locals.len() {
                        locals.resize(*slot + 1, Value::Unit);
                    }
                    locals[*slot] = v;
                }
                Instr::Pop => {
                    if stack.pop().is_none() {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "Stack underflow on Pop",
                            function_name,
                            ip,
                        ));
                    }
                }
                Instr::NegInt => {
                    let Some(v) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "NegInt expects value",
                            function_name,
                            ip,
                        ));
                    };
                    match v {
                        Value::Int(v) => stack.push(Value::Int(-v)),
                        Value::Float(v) => stack.push(Value::Float(-v)),
                        _ => {
                            return Err(Self::err_at(
                                VmErrorKind::TypeMismatch,
                                "NegInt expects Int or Float",
                                function_name,
                                ip,
                            ));
                        }
                    }
                }
                Instr::NotBool => {
                    let Some(Value::Bool(v)) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "NotBool expects Bool",
                            function_name,
                            ip,
                        ));
                    };
                    stack.push(Value::Bool(!v));
                }
                Instr::Add => {
                    let Some(r) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "Add expects rhs",
                            function_name,
                            ip,
                        ));
                    };
                    let Some(l) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "Add expects lhs",
                            function_name,
                            ip,
                        ));
                    };
                    match (l, r) {
                        (Value::Int(a), Value::Int(b)) => stack.push(Value::Int(a + b)),
                        (Value::Float(a), Value::Float(b)) => stack.push(Value::Float(a + b)),
                        (Value::String(a), Value::String(b)) => {
                            stack.push(Value::String(format!("{a}{b}")))
                        }
                        (Value::Array(mut a), Value::Array(b)) => {
                            a.extend(b);
                            stack.push(Value::Array(a));
                        }
                        _ => {
                            return Err(Self::err_at(
                                VmErrorKind::TypeMismatch,
                                "Add supports Int+Int, Float+Float, String+String, or Array+Array",
                                function_name,
                                ip,
                            ));
                        }
                    }
                }
                Instr::SubInt
                | Instr::MulInt
                | Instr::DivInt
                | Instr::LtInt
                | Instr::LteInt
                | Instr::GtInt
                | Instr::GteInt => {
                    let Some(r) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "int binary op expects rhs",
                            function_name,
                            ip,
                        ));
                    };
                    let Some(l) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "int binary op expects lhs",
                            function_name,
                            ip,
                        ));
                    };
                    match (l, r) {
                        (Value::Int(l), Value::Int(r)) => match chunk.code[ip] {
                            Instr::SubInt => stack.push(Value::Int(l - r)),
                            Instr::MulInt => stack.push(Value::Int(l * r)),
                            Instr::DivInt => {
                                if r == 0 {
                                    return Err(Self::err_at(
                                        VmErrorKind::DivisionByZero,
                                        "division by zero",
                                        function_name,
                                        ip,
                                    ));
                                }
                                stack.push(Value::Int(l / r));
                            }
                            Instr::LtInt => stack.push(Value::Bool(l < r)),
                            Instr::LteInt => stack.push(Value::Bool(l <= r)),
                            Instr::GtInt => stack.push(Value::Bool(l > r)),
                            Instr::GteInt => stack.push(Value::Bool(l >= r)),
                            _ => unreachable!(),
                        },
                        (Value::Float(l), Value::Float(r)) => match chunk.code[ip] {
                            Instr::SubInt => stack.push(Value::Float(l - r)),
                            Instr::MulInt => stack.push(Value::Float(l * r)),
                            Instr::DivInt => {
                                if r == 0.0 {
                                    return Err(Self::err_at(
                                        VmErrorKind::DivisionByZero,
                                        "division by zero",
                                        function_name,
                                        ip,
                                    ));
                                }
                                stack.push(Value::Float(l / r));
                            }
                            Instr::LtInt => stack.push(Value::Bool(l < r)),
                            Instr::LteInt => stack.push(Value::Bool(l <= r)),
                            Instr::GtInt => stack.push(Value::Bool(l > r)),
                            Instr::GteInt => stack.push(Value::Bool(l >= r)),
                            _ => unreachable!(),
                        },
                        _ => {
                            return Err(Self::err_at(
                                VmErrorKind::TypeMismatch,
                                "numeric binary op expects matching Int/Float operands",
                                function_name,
                                ip,
                            ));
                        }
                    }
                }
                Instr::ModInt => {
                    let Some(Value::Int(r)) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "ModInt expects rhs Int",
                            function_name,
                            ip,
                        ));
                    };
                    let Some(Value::Int(l)) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "ModInt expects lhs Int",
                            function_name,
                            ip,
                        ));
                    };
                    if r == 0 {
                        return Err(Self::err_at(
                            VmErrorKind::DivisionByZero,
                            "modulo by zero",
                            function_name,
                            ip,
                        ));
                    }
                    stack.push(Value::Int(l % r));
                }
                Instr::Eq => {
                    let Some(r) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "Eq expects rhs",
                            function_name,
                            ip,
                        ));
                    };
                    let Some(l) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "Eq expects lhs",
                            function_name,
                            ip,
                        ));
                    };
                    stack.push(Value::Bool(l == r));
                }
                Instr::Neq => {
                    let Some(r) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "Neq expects rhs",
                            function_name,
                            ip,
                        ));
                    };
                    let Some(l) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "Neq expects lhs",
                            function_name,
                            ip,
                        ));
                    };
                    stack.push(Value::Bool(l != r));
                }
                Instr::AndBool | Instr::OrBool => {
                    let Some(Value::Bool(r)) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "logical op expects rhs Bool",
                            function_name,
                            ip,
                        ));
                    };
                    let Some(Value::Bool(l)) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "logical op expects lhs Bool",
                            function_name,
                            ip,
                        ));
                    };
                    match chunk.code[ip] {
                        Instr::AndBool => stack.push(Value::Bool(l && r)),
                        Instr::OrBool => stack.push(Value::Bool(l || r)),
                        _ => unreachable!(),
                    }
                }
                Instr::Jump(target) => {
                    ip = *target;
                    continue;
                }
                Instr::JumpIfFalse(target) => {
                    let Some(Value::Bool(v)) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "JumpIfFalse expects Bool",
                            function_name,
                            ip,
                        ));
                    };
                    if !v {
                        ip = *target;
                        continue;
                    }
                }
                Instr::JumpIfTrue(target) => {
                    let Some(Value::Bool(v)) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "JumpIfTrue expects Bool",
                            function_name,
                            ip,
                        ));
                    };
                    if v {
                        ip = *target;
                        continue;
                    }
                }
                Instr::Call {
                    name: callee_name,
                    argc,
                } => {
                    if stack.len() < *argc {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "Stack underflow on Call",
                            function_name,
                            ip,
                        ));
                    }
                    let split = stack.len() - *argc;
                    let call_args = stack.split_off(split);
                    let ret = Self::run_function(
                        module,
                        callee_name,
                        call_args,
                        host,
                        reg,
                        depth + 1,
                        config,
                    )?;
                    stack.push(ret);
                }
                Instr::CallBuiltin {
                    package,
                    name,
                    argc,
                } => {
                    if stack.len() < *argc {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "Stack underflow on CallBuiltin",
                            function_name,
                            ip,
                        ));
                    }
                    let split = stack.len() - *argc;
                    let call_args = stack.split_off(split);
                    let ret = reg.call(host, package, name, call_args)?;
                    stack.push(ret);
                }
                Instr::MakeArray(n) => {
                    if stack.len() < *n {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "MakeArray expects enough stack values",
                            function_name,
                            ip,
                        ));
                    }
                    let start = stack.len() - *n;
                    let items = stack.split_off(start);
                    stack.push(Value::Array(items));
                }
                Instr::MakeArrayRepeat(n) => {
                    let Some(v) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "MakeArrayRepeat expects a value",
                            function_name,
                            ip,
                        ));
                    };
                    stack.push(Value::Array(vec![v; *n]));
                }
                Instr::ArrayGet => {
                    let Some(idx_v) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "ArrayGet expects index",
                            function_name,
                            ip,
                        ));
                    };
                    let Some(arr_v) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "ArrayGet expects array",
                            function_name,
                            ip,
                        ));
                    };
                    let Value::Int(idx) = idx_v else {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "ArrayGet index must be Int",
                            function_name,
                            ip,
                        ));
                    };
                    let Value::Array(items) = arr_v else {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "ArrayGet expects Array",
                            function_name,
                            ip,
                        ));
                    };
                    if idx < 0 || idx as usize >= items.len() {
                        return Err(Self::err_at(
                            VmErrorKind::IndexOutOfBounds,
                            format!("Array index {} out of bounds (len={})", idx, items.len()),
                            function_name,
                            ip,
                        ));
                    }
                    stack.push(items[idx as usize].clone());
                }
                Instr::ArraySet => {
                    let Some(val) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "ArraySet expects value",
                            function_name,
                            ip,
                        ));
                    };
                    let Some(idx_v) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "ArraySet expects index",
                            function_name,
                            ip,
                        ));
                    };
                    let Some(arr_v) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "ArraySet expects array",
                            function_name,
                            ip,
                        ));
                    };
                    let Value::Int(idx) = idx_v else {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "ArraySet index must be Int",
                            function_name,
                            ip,
                        ));
                    };
                    let Value::Array(mut items) = arr_v else {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "ArraySet expects Array",
                            function_name,
                            ip,
                        ));
                    };
                    if idx < 0 || idx as usize >= items.len() {
                        return Err(Self::err_at(
                            VmErrorKind::IndexOutOfBounds,
                            format!("Array index {} out of bounds (len={})", idx, items.len()),
                            function_name,
                            ip,
                        ));
                    }
                    items[idx as usize] = val;
                    stack.push(Value::Array(items));
                }
                Instr::ArraySetChain(depth) => {
                    if *depth == 0 {
                        return Err(Self::err_at(
                            VmErrorKind::TypeMismatch,
                            "ArraySetChain depth must be > 0",
                            function_name,
                            ip,
                        ));
                    }
                    let Some(val) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "ArraySetChain expects value",
                            function_name,
                            ip,
                        ));
                    };
                    if stack.len() < *depth + 1 {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "ArraySetChain expects array and all indices",
                            function_name,
                            ip,
                        ));
                    }
                    let mut indices = Vec::with_capacity(*depth);
                    for _ in 0..*depth {
                        let Some(idx_v) = stack.pop() else {
                            return Err(Self::err_at(
                                VmErrorKind::StackUnderflow,
                                "ArraySetChain expects index",
                                function_name,
                                ip,
                            ));
                        };
                        let Value::Int(idx) = idx_v else {
                            return Err(Self::err_at(
                                VmErrorKind::TypeMismatch,
                                "ArraySetChain index must be Int",
                                function_name,
                                ip,
                            ));
                        };
                        indices.push(idx);
                    }
                    indices.reverse();
                    let Some(arr_v) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "ArraySetChain expects array",
                            function_name,
                            ip,
                        ));
                    };

                    fn set_deep(
                        cur: Value,
                        indices: &[i64],
                        val: Value,
                    ) -> Result<Value, VmErrorKind> {
                        let Value::Array(mut items) = cur else {
                            return Err(VmErrorKind::TypeMismatch);
                        };
                        let idx = indices[0];
                        if idx < 0 || idx as usize >= items.len() {
                            return Err(VmErrorKind::IndexOutOfBounds);
                        }
                        let u = idx as usize;
                        if indices.len() == 1 {
                            items[u] = val;
                            return Ok(Value::Array(items));
                        }
                        let child = items[u].clone();
                        let next = set_deep(child, &indices[1..], val)?;
                        items[u] = next;
                        Ok(Value::Array(items))
                    }

                    match set_deep(arr_v, &indices, val) {
                        Ok(updated) => stack.push(updated),
                        Err(VmErrorKind::TypeMismatch) => {
                            return Err(Self::err_at(
                                VmErrorKind::TypeMismatch,
                                "ArraySetChain expects nested arrays along the assignment path",
                                function_name,
                                ip,
                            ));
                        }
                        Err(VmErrorKind::IndexOutOfBounds) => {
                            return Err(Self::err_at(
                                VmErrorKind::IndexOutOfBounds,
                                "ArraySetChain index out of bounds",
                                function_name,
                                ip,
                            ));
                        }
                        Err(other) => {
                            return Err(Self::err_at(
                                other,
                                "ArraySetChain failed",
                                function_name,
                                ip,
                            ));
                        }
                    }
                }
                Instr::ArrayLen => {
                    let Some(arr_v) = stack.pop() else {
                        return Err(Self::err_at(
                            VmErrorKind::StackUnderflow,
                            "ArrayLen expects value",
                            function_name,
                            ip,
                        ));
                    };
                    match arr_v {
                        Value::Array(items) => stack.push(Value::Int(items.len() as i64)),
                        Value::String(s) => stack.push(Value::Int(s.chars().count() as i64)),
                        _ => {
                            return Err(Self::err_at(
                                VmErrorKind::TypeMismatch,
                                "len expects Array or String",
                                function_name,
                                ip,
                            ));
                        }
                    }
                }
                Instr::Return => {
                    return Ok(stack.pop().unwrap_or(Value::Unit));
                }
            }
            ip += 1;
        }

        Ok(Value::Unit)
    }

    fn err_at(kind: VmErrorKind, message: impl Into<String>, function: &str, ip: usize) -> VmError {
        let msg = message.into();
        VmError::new(kind, format!("{function}@{ip}: {msg}"))
    }
}
