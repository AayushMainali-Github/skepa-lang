mod error;
mod builtins;
mod host;

use crate::bytecode::{BytecodeModule, FunctionChunk, Instr, Value};
pub use error::{VmError, VmErrorKind};
pub use host::{StdIoHost, TestHost};

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

pub trait BuiltinHost {
    fn write(&mut self, s: &str, newline: bool) -> Result<(), VmError>;
    fn read_line(&mut self) -> Result<String, VmError>;
}

pub use builtins::{BuiltinHandler, BuiltinRegistry};

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
