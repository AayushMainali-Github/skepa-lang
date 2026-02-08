use std::collections::{HashMap, VecDeque};
use std::io::{self, Write};

use crate::bytecode::{BytecodeModule, FunctionChunk, Instr, Value};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vm;

pub trait BuiltinHost {
    fn write(&mut self, s: &str, newline: bool) -> Result<(), String>;
    fn read_line(&mut self) -> Result<String, String>;
}

pub struct StdIoHost;

impl BuiltinHost for StdIoHost {
    fn write(&mut self, s: &str, newline: bool) -> Result<(), String> {
        if newline {
            println!("{s}");
        } else {
            print!("{s}");
            io::stdout().flush().map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn read_line(&mut self) -> Result<String, String> {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).map_err(|e| e.to_string())?;
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
    fn write(&mut self, s: &str, newline: bool) -> Result<(), String> {
        self.output.push_str(s);
        if newline {
            self.output.push('\n');
        }
        Ok(())
    }

    fn read_line(&mut self) -> Result<String, String> {
        Ok(self.input.pop_front().unwrap_or_default())
    }
}

type BuiltinHandler = fn(&mut dyn BuiltinHost, Vec<Value>) -> Result<Value, String>;

#[derive(Default)]
struct BuiltinRegistry {
    handlers: HashMap<String, BuiltinHandler>,
}

impl BuiltinRegistry {
    fn with_defaults() -> Self {
        let mut r = Self::default();
        r.register("io", "print", builtin_io_print);
        r.register("io", "println", builtin_io_println);
        r.register("io", "readLine", builtin_io_readline);
        r
    }

    fn key(package: &str, name: &str) -> String {
        format!("{package}.{name}")
    }

    fn register(&mut self, package: &str, name: &str, handler: BuiltinHandler) {
        self.handlers.insert(Self::key(package, name), handler);
    }

    fn call(
        &self,
        host: &mut dyn BuiltinHost,
        package: &str,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let key = Self::key(package, name);
        let Some(handler) = self.handlers.get(&key).copied() else {
            return Err(format!("Unknown builtin `{key}`"));
        };
        handler(host, args)
    }
}

fn builtin_io_print(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("io.print expects 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => host.write(s, false)?,
        _ => return Err("io.print expects String argument".to_string()),
    }
    Ok(Value::Unit)
}

fn builtin_io_println(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("io.println expects 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => host.write(s, true)?,
        _ => return Err("io.println expects String argument".to_string()),
    }
    Ok(Value::Unit)
}

fn builtin_io_readline(host: &mut dyn BuiltinHost, args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("io.readLine expects 0 arguments".to_string());
    }
    Ok(Value::String(host.read_line()?))
}

impl Vm {
    pub fn run_module_main(module: &BytecodeModule) -> Result<Value, String> {
        let mut host = StdIoHost;
        let reg = BuiltinRegistry::with_defaults();
        Self::run_function(module, "main", Vec::new(), &mut host, &reg)
    }

    pub fn run_module_main_with_host(
        module: &BytecodeModule,
        host: &mut dyn BuiltinHost,
    ) -> Result<Value, String> {
        let reg = BuiltinRegistry::with_defaults();
        Self::run_function(module, "main", Vec::new(), host, &reg)
    }

    pub fn run_main(chunk: &FunctionChunk) -> Result<Value, String> {
        let module = BytecodeModule {
            functions: vec![(chunk.name.clone(), chunk.clone())].into_iter().collect(),
        };
        Self::run_module_main(&module)
    }

    fn run_function(
        module: &BytecodeModule,
        name: &str,
        args: Vec<Value>,
        host: &mut dyn BuiltinHost,
        reg: &BuiltinRegistry,
    ) -> Result<Value, String> {
        let Some(chunk) = module.functions.get(name) else {
            return Err(format!("Unknown function `{name}`"));
        };
        if args.len() != chunk.param_count {
            return Err(format!(
                "Function `{}` arity mismatch: expected {}, got {}",
                name, chunk.param_count, args.len()
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
            match &chunk.code[ip] {
                Instr::LoadConst(v) => stack.push(v.clone()),
                Instr::LoadLocal(slot) => {
                    let Some(v) = locals.get(*slot).cloned() else {
                        return Err(format!("Invalid local slot {slot}"));
                    };
                    stack.push(v);
                }
                Instr::StoreLocal(slot) => {
                    let Some(v) = stack.pop() else {
                        return Err("Stack underflow on StoreLocal".to_string());
                    };
                    if *slot >= locals.len() {
                        locals.resize(*slot + 1, Value::Unit);
                    }
                    locals[*slot] = v;
                }
                Instr::Pop => {
                    if stack.pop().is_none() {
                        return Err("Stack underflow on Pop".to_string());
                    }
                }
                Instr::NegInt => {
                    let Some(Value::Int(v)) = stack.pop() else {
                        return Err("NegInt expects Int".to_string());
                    };
                    stack.push(Value::Int(-v));
                }
                Instr::NotBool => {
                    let Some(Value::Bool(v)) = stack.pop() else {
                        return Err("NotBool expects Bool".to_string());
                    };
                    stack.push(Value::Bool(!v));
                }
                Instr::Add => {
                    let Some(r) = stack.pop() else {
                        return Err("Add expects rhs".to_string());
                    };
                    let Some(l) = stack.pop() else {
                        return Err("Add expects lhs".to_string());
                    };
                    match (l, r) {
                        (Value::Int(a), Value::Int(b)) => stack.push(Value::Int(a + b)),
                        (Value::String(a), Value::String(b)) => {
                            stack.push(Value::String(format!("{a}{b}")))
                        }
                        _ => return Err("Add supports Int+Int or String+String".to_string()),
                    }
                }
                Instr::SubInt | Instr::MulInt | Instr::DivInt | Instr::LtInt | Instr::LteInt | Instr::GtInt | Instr::GteInt => {
                    let Some(Value::Int(r)) = stack.pop() else {
                        return Err("int binary op expects rhs Int".to_string());
                    };
                    let Some(Value::Int(l)) = stack.pop() else {
                        return Err("int binary op expects lhs Int".to_string());
                    };
                    match chunk.code[ip] {
                        Instr::SubInt => stack.push(Value::Int(l - r)),
                        Instr::MulInt => stack.push(Value::Int(l * r)),
                        Instr::DivInt => {
                            if r == 0 {
                                return Err("division by zero".to_string());
                            }
                            stack.push(Value::Int(l / r));
                        }
                        Instr::LtInt => stack.push(Value::Bool(l < r)),
                        Instr::LteInt => stack.push(Value::Bool(l <= r)),
                        Instr::GtInt => stack.push(Value::Bool(l > r)),
                        Instr::GteInt => stack.push(Value::Bool(l >= r)),
                        _ => unreachable!(),
                    }
                }
                Instr::Eq => {
                    let Some(r) = stack.pop() else { return Err("Eq expects rhs".to_string()); };
                    let Some(l) = stack.pop() else { return Err("Eq expects lhs".to_string()); };
                    stack.push(Value::Bool(l == r));
                }
                Instr::Neq => {
                    let Some(r) = stack.pop() else { return Err("Neq expects rhs".to_string()); };
                    let Some(l) = stack.pop() else { return Err("Neq expects lhs".to_string()); };
                    stack.push(Value::Bool(l != r));
                }
                Instr::AndBool | Instr::OrBool => {
                    let Some(Value::Bool(r)) = stack.pop() else {
                        return Err("logical op expects rhs Bool".to_string());
                    };
                    let Some(Value::Bool(l)) = stack.pop() else {
                        return Err("logical op expects lhs Bool".to_string());
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
                        return Err("JumpIfFalse expects Bool".to_string());
                    };
                    if !v {
                        ip = *target;
                        continue;
                    }
                }
                Instr::Call { name, argc } => {
                    if stack.len() < *argc {
                        return Err("Stack underflow on Call".to_string());
                    }
                    let split = stack.len() - *argc;
                    let call_args = stack.split_off(split);
                    let ret = Self::run_function(module, name, call_args, host, reg)?;
                    stack.push(ret);
                }
                Instr::CallBuiltin {
                    package,
                    name,
                    argc,
                } => {
                    if stack.len() < *argc {
                        return Err("Stack underflow on CallBuiltin".to_string());
                    }
                    let split = stack.len() - *argc;
                    let call_args = stack.split_off(split);
                    let ret = reg.call(host, package, name, call_args)?;
                    stack.push(ret);
                }
                Instr::Return => {
                    return Ok(stack.pop().unwrap_or(Value::Unit));
                }
            }
            ip += 1;
        }

        Ok(Value::Unit)
    }
}
