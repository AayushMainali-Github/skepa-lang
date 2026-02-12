use std::collections::HashMap;

use crate::ast::{AssignTarget, BinaryOp, Expr, Program, Stmt, UnaryOp};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::parser::Parser;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    LoadConst(Value),
    LoadLocal(usize),
    StoreLocal(usize),
    Pop,
    NegInt,
    NotBool,
    Add,
    SubInt,
    MulInt,
    DivInt,
    ModInt,
    Eq,
    Neq,
    LtInt,
    LteInt,
    GtInt,
    GteInt,
    AndBool,
    OrBool,
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    Call {
        name: String,
        argc: usize,
    },
    CallBuiltin {
        package: String,
        name: String,
        argc: usize,
    },
    Return,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FunctionChunk {
    pub name: String,
    pub code: Vec<Instr>,
    pub locals_count: usize,
    pub param_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BytecodeModule {
    pub functions: HashMap<String, FunctionChunk>,
}

impl BytecodeModule {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"SKBC");
        write_u32(&mut out, 1);
        write_u32(&mut out, self.functions.len() as u32);
        let mut funcs: Vec<_> = self.functions.values().collect();
        funcs.sort_by(|a, b| a.name.cmp(&b.name));
        for f in funcs {
            write_str(&mut out, &f.name);
            write_u32(&mut out, f.locals_count as u32);
            write_u32(&mut out, f.param_count as u32);
            write_u32(&mut out, f.code.len() as u32);
            for instr in &f.code {
                encode_instr(instr, &mut out);
            }
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut rd = Reader { bytes, idx: 0 };
        let magic = rd.read_exact(4)?;
        if magic != b"SKBC" {
            return Err("Invalid bytecode magic header".to_string());
        }
        let version = rd.read_u32()?;
        if version != 1 {
            return Err(format!("Unsupported bytecode version {version}"));
        }
        let funcs_len = rd.read_u32()? as usize;
        let mut functions = HashMap::new();
        for _ in 0..funcs_len {
            let name = rd.read_str()?;
            let locals_count = rd.read_u32()? as usize;
            let param_count = rd.read_u32()? as usize;
            let code_len = rd.read_u32()? as usize;
            let mut code = Vec::with_capacity(code_len);
            for _ in 0..code_len {
                code.push(decode_instr(&mut rd)?);
            }
            functions.insert(
                name.clone(),
                FunctionChunk {
                    name,
                    code,
                    locals_count,
                    param_count,
                },
            );
        }
        Ok(Self { functions })
    }

    pub fn disassemble(&self) -> String {
        let mut out = String::new();
        let mut funcs: Vec<_> = self.functions.values().collect();
        funcs.sort_by(|a, b| a.name.cmp(&b.name));
        for f in funcs {
            out.push_str(&format!(
                "fn {} (params={}, locals={})\n",
                f.name, f.param_count, f.locals_count
            ));
            for (ip, instr) in f.code.iter().enumerate() {
                out.push_str(&format!("  {:04} {}\n", ip, fmt_instr(instr)));
            }
        }
        out
    }
}

pub fn compile_source(source: &str) -> Result<BytecodeModule, DiagnosticBag> {
    let (program, mut diags) = Parser::parse_source(source);
    if !diags.is_empty() {
        return Err(diags);
    }

    let mut compiler = Compiler::default();
    let module = compiler.compile_program(&program);
    for d in compiler.diags.into_vec() {
        diags.push(d);
    }

    if diags.is_empty() {
        Ok(module)
    } else {
        Err(diags)
    }
}

#[derive(Default)]
struct Compiler {
    diags: DiagnosticBag,
}

impl Compiler {
    fn compile_program(&mut self, program: &Program) -> BytecodeModule {
        let mut module = BytecodeModule::default();
        for func in &program.functions {
            let chunk = self.compile_function(func);
            module.functions.insert(func.name.clone(), chunk);
        }
        module
    }

    fn compile_function(&mut self, func: &crate::ast::FnDecl) -> FunctionChunk {
        let mut ctx = FnCtx::default();
        let mut loops: Vec<LoopCtx> = Vec::new();
        let mut code = Vec::new();

        for param in &func.params {
            ctx.alloc_local(param.name.clone());
        }

        for stmt in &func.body {
            self.compile_stmt(stmt, &mut ctx, &mut loops, &mut code);
        }

        if !matches!(code.last(), Some(Instr::Return)) {
            code.push(Instr::LoadConst(Value::Unit));
            code.push(Instr::Return);
        }

        FunctionChunk {
            name: func.name.clone(),
            code,
            locals_count: ctx.next_local,
            param_count: func.params.len(),
        }
    }

    fn compile_stmt(
        &mut self,
        stmt: &Stmt,
        ctx: &mut FnCtx,
        loops: &mut Vec<LoopCtx>,
        code: &mut Vec<Instr>,
    ) {
        match stmt {
            Stmt::Let { name, value, .. } => {
                self.compile_expr(value, ctx, code);
                let slot = ctx.alloc_local(name.clone());
                code.push(Instr::StoreLocal(slot));
            }
            Stmt::Assign { target, value } => {
                self.compile_expr(value, ctx, code);
                match target {
                    AssignTarget::Ident(name) => {
                        if let Some(slot) = ctx.lookup(name) {
                            code.push(Instr::StoreLocal(slot));
                        } else {
                            self.error(format!("Unknown local `{name}` in assignment"));
                        }
                    }
                    AssignTarget::Path(parts) => {
                        self.error(format!(
                            "Path assignment not supported in bytecode v0: {}",
                            parts.join(".")
                        ));
                    }
                }
            }
            Stmt::Expr(expr) => {
                self.compile_expr(expr, ctx, code);
                code.push(Instr::Pop);
            }
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    self.compile_expr(expr, ctx, code);
                } else {
                    code.push(Instr::LoadConst(Value::Unit));
                }
                code.push(Instr::Return);
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => {
                self.compile_expr(cond, ctx, code);
                let jmp_false_at = code.len();
                code.push(Instr::JumpIfFalse(usize::MAX));

                for s in then_body {
                    self.compile_stmt(s, ctx, loops, code);
                }

                if else_body.is_empty() {
                    let after_then = code.len();
                    code[jmp_false_at] = Instr::JumpIfFalse(after_then);
                } else {
                    let jmp_end_at = code.len();
                    code.push(Instr::Jump(usize::MAX));

                    let else_start = code.len();
                    code[jmp_false_at] = Instr::JumpIfFalse(else_start);

                    for s in else_body {
                        self.compile_stmt(s, ctx, loops, code);
                    }

                    let end = code.len();
                    code[jmp_end_at] = Instr::Jump(end);
                }
            }
            Stmt::While { cond, body } => {
                let loop_start = code.len();
                loops.push(LoopCtx {
                    loop_start,
                    break_jumps: Vec::new(),
                });
                self.compile_expr(cond, ctx, code);
                let jmp_false_at = code.len();
                code.push(Instr::JumpIfFalse(usize::MAX));

                for s in body {
                    self.compile_stmt(s, ctx, loops, code);
                }

                code.push(Instr::Jump(loop_start));
                let loop_end = code.len();
                code[jmp_false_at] = Instr::JumpIfFalse(loop_end);
                if let Some(lp) = loops.pop() {
                    for at in lp.break_jumps {
                        code[at] = Instr::Jump(loop_end);
                    }
                }
            }
            Stmt::Break => {
                if let Some(lp) = loops.last_mut() {
                    let at = code.len();
                    code.push(Instr::Jump(usize::MAX));
                    lp.break_jumps.push(at);
                } else {
                    self.error("`break` used outside while loop".to_string());
                }
            }
            Stmt::Continue => {
                if let Some(lp) = loops.last() {
                    code.push(Instr::Jump(lp.loop_start));
                } else {
                    self.error("`continue` used outside while loop".to_string());
                }
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr, ctx: &mut FnCtx, code: &mut Vec<Instr>) {
        match expr {
            Expr::IntLit(v) => code.push(Instr::LoadConst(Value::Int(*v))),
            Expr::FloatLit(v) => {
                if let Ok(n) = v.parse::<f64>() {
                    code.push(Instr::LoadConst(Value::Float(n)));
                } else {
                    self.error(format!("Invalid float literal `{v}`"));
                    code.push(Instr::LoadConst(Value::Float(0.0)));
                }
            }
            Expr::BoolLit(v) => code.push(Instr::LoadConst(Value::Bool(*v))),
            Expr::StringLit(v) => code.push(Instr::LoadConst(Value::String(v.clone()))),
            Expr::Ident(name) => {
                if let Some(slot) = ctx.lookup(name) {
                    code.push(Instr::LoadLocal(slot));
                } else {
                    self.error(format!("Unknown local `{name}`"));
                    code.push(Instr::LoadConst(Value::Int(0)));
                }
            }
            Expr::Unary { op, expr } => match op {
                UnaryOp::Neg => {
                    self.compile_expr(expr, ctx, code);
                    code.push(Instr::NegInt);
                }
                UnaryOp::Pos => {
                    self.compile_expr(expr, ctx, code);
                }
                UnaryOp::Not => {
                    self.compile_expr(expr, ctx, code);
                    code.push(Instr::NotBool);
                }
            },
            Expr::Binary { left, op, right } => match op {
                BinaryOp::AndAnd => {
                    self.compile_expr(left, ctx, code);
                    let jmp_false_at = code.len();
                    code.push(Instr::JumpIfFalse(usize::MAX));
                    self.compile_expr(right, ctx, code);
                    let jmp_end_at = code.len();
                    code.push(Instr::Jump(usize::MAX));
                    let false_label = code.len();
                    code.push(Instr::LoadConst(Value::Bool(false)));
                    let end_label = code.len();
                    code[jmp_false_at] = Instr::JumpIfFalse(false_label);
                    code[jmp_end_at] = Instr::Jump(end_label);
                }
                BinaryOp::OrOr => {
                    self.compile_expr(left, ctx, code);
                    let jmp_true_at = code.len();
                    code.push(Instr::JumpIfTrue(usize::MAX));
                    self.compile_expr(right, ctx, code);
                    let jmp_end_at = code.len();
                    code.push(Instr::Jump(usize::MAX));
                    let true_label = code.len();
                    code.push(Instr::LoadConst(Value::Bool(true)));
                    let end_label = code.len();
                    code[jmp_true_at] = Instr::JumpIfTrue(true_label);
                    code[jmp_end_at] = Instr::Jump(end_label);
                }
                _ => {
                    self.compile_expr(left, ctx, code);
                    self.compile_expr(right, ctx, code);
                    match op {
                        BinaryOp::Add => code.push(Instr::Add),
                        BinaryOp::Sub => code.push(Instr::SubInt),
                        BinaryOp::Mul => code.push(Instr::MulInt),
                        BinaryOp::Div => code.push(Instr::DivInt),
                        BinaryOp::Mod => code.push(Instr::ModInt),
                        BinaryOp::EqEq => code.push(Instr::Eq),
                        BinaryOp::Neq => code.push(Instr::Neq),
                        BinaryOp::Lt => code.push(Instr::LtInt),
                        BinaryOp::Lte => code.push(Instr::LteInt),
                        BinaryOp::Gt => code.push(Instr::GtInt),
                        BinaryOp::Gte => code.push(Instr::GteInt),
                        BinaryOp::AndAnd | BinaryOp::OrOr => unreachable!(),
                    }
                }
            },
            Expr::Call { callee, args } => {
                if let Expr::Path(parts) = &**callee {
                    if parts.len() == 2 {
                        for arg in args {
                            self.compile_expr(arg, ctx, code);
                        }
                        code.push(Instr::CallBuiltin {
                            package: parts[0].clone(),
                            name: parts[1].clone(),
                            argc: args.len(),
                        });
                        return;
                    }
                    self.error("Only `package.function(...)` builtins are supported".to_string());
                    return;
                }

                let name = match &**callee {
                    Expr::Ident(name) => name.clone(),
                    _ => {
                        self.error(
                            "Only direct function calls are supported in bytecode v0 slice"
                                .to_string(),
                        );
                        return;
                    }
                };
                for arg in args {
                    self.compile_expr(arg, ctx, code);
                }
                code.push(Instr::Call {
                    name,
                    argc: args.len(),
                });
            }
            Expr::Group(inner) => self.compile_expr(inner, ctx, code),
            Expr::Path(_) => {
                self.error(
                    "Path expression value is not supported in bytecode v0 compiler slice"
                        .to_string(),
                );
            }
        }
    }

    fn error(&mut self, message: String) {
        self.diags.error(message, Span::default());
    }
}

#[derive(Debug, Clone, Default)]
struct LoopCtx {
    loop_start: usize,
    break_jumps: Vec<usize>,
}

#[derive(Default)]
struct FnCtx {
    locals: HashMap<String, usize>,
    next_local: usize,
}

impl FnCtx {
    fn alloc_local(&mut self, name: String) -> usize {
        let slot = self.next_local;
        self.next_local += 1;
        self.locals.insert(name, slot);
        slot
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.locals.get(name).copied()
    }
}

fn write_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}
fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_i64(out: &mut Vec<u8>, v: i64) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_f64(out: &mut Vec<u8>, v: f64) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_bool(out: &mut Vec<u8>, v: bool) {
    write_u8(out, if v { 1 } else { 0 });
}
fn write_str(out: &mut Vec<u8>, s: &str) {
    write_u32(out, s.len() as u32);
    out.extend_from_slice(s.as_bytes());
}

fn encode_value(v: &Value, out: &mut Vec<u8>) {
    match v {
        Value::Int(n) => {
            write_u8(out, 0);
            write_i64(out, *n);
        }
        Value::Float(n) => {
            write_u8(out, 1);
            write_f64(out, *n);
        }
        Value::Bool(b) => {
            write_u8(out, 2);
            write_bool(out, *b);
        }
        Value::String(s) => {
            write_u8(out, 3);
            write_str(out, s);
        }
        Value::Unit => write_u8(out, 4),
    }
}

fn encode_instr(i: &Instr, out: &mut Vec<u8>) {
    match i {
        Instr::LoadConst(v) => {
            write_u8(out, 0);
            encode_value(v, out);
        }
        Instr::LoadLocal(s) => {
            write_u8(out, 1);
            write_u32(out, *s as u32);
        }
        Instr::StoreLocal(s) => {
            write_u8(out, 2);
            write_u32(out, *s as u32);
        }
        Instr::Pop => write_u8(out, 3),
        Instr::NegInt => write_u8(out, 4),
        Instr::NotBool => write_u8(out, 5),
        Instr::Add => write_u8(out, 6),
        Instr::SubInt => write_u8(out, 7),
        Instr::MulInt => write_u8(out, 8),
        Instr::DivInt => write_u8(out, 9),
        Instr::ModInt => write_u8(out, 10),
        Instr::Eq => write_u8(out, 11),
        Instr::Neq => write_u8(out, 12),
        Instr::LtInt => write_u8(out, 13),
        Instr::LteInt => write_u8(out, 14),
        Instr::GtInt => write_u8(out, 15),
        Instr::GteInt => write_u8(out, 16),
        Instr::AndBool => write_u8(out, 17),
        Instr::OrBool => write_u8(out, 18),
        Instr::Jump(t) => {
            write_u8(out, 19);
            write_u32(out, *t as u32);
        }
        Instr::JumpIfFalse(t) => {
            write_u8(out, 20);
            write_u32(out, *t as u32);
        }
        Instr::JumpIfTrue(t) => {
            write_u8(out, 21);
            write_u32(out, *t as u32);
        }
        Instr::Call { name, argc } => {
            write_u8(out, 22);
            write_str(out, name);
            write_u32(out, *argc as u32);
        }
        Instr::CallBuiltin {
            package,
            name,
            argc,
        } => {
            write_u8(out, 23);
            write_str(out, package);
            write_str(out, name);
            write_u32(out, *argc as u32);
        }
        Instr::Return => write_u8(out, 24),
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    idx: usize,
}
impl<'a> Reader<'a> {
    fn read_exact(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.idx + n > self.bytes.len() {
            return Err("Unexpected EOF while decoding bytecode".to_string());
        }
        let s = &self.bytes[self.idx..self.idx + n];
        self.idx += n;
        Ok(s)
    }
    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_exact(1)?[0])
    }
    fn read_u32(&mut self) -> Result<u32, String> {
        let b = self.read_exact(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn read_i64(&mut self) -> Result<i64, String> {
        let b = self.read_exact(8)?;
        Ok(i64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
    fn read_f64(&mut self) -> Result<f64, String> {
        let b = self.read_exact(8)?;
        Ok(f64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
    fn read_bool(&mut self) -> Result<bool, String> {
        Ok(self.read_u8()? != 0)
    }
    fn read_str(&mut self) -> Result<String, String> {
        let n = self.read_u32()? as usize;
        let b = self.read_exact(n)?;
        String::from_utf8(b.to_vec()).map_err(|e| e.to_string())
    }
}

fn decode_value(rd: &mut Reader<'_>) -> Result<Value, String> {
    match rd.read_u8()? {
        0 => Ok(Value::Int(rd.read_i64()?)),
        1 => Ok(Value::Float(rd.read_f64()?)),
        2 => Ok(Value::Bool(rd.read_bool()?)),
        3 => Ok(Value::String(rd.read_str()?)),
        4 => Ok(Value::Unit),
        t => Err(format!("Unknown value tag {t}")),
    }
}

fn decode_instr(rd: &mut Reader<'_>) -> Result<Instr, String> {
    Ok(match rd.read_u8()? {
        0 => Instr::LoadConst(decode_value(rd)?),
        1 => Instr::LoadLocal(rd.read_u32()? as usize),
        2 => Instr::StoreLocal(rd.read_u32()? as usize),
        3 => Instr::Pop,
        4 => Instr::NegInt,
        5 => Instr::NotBool,
        6 => Instr::Add,
        7 => Instr::SubInt,
        8 => Instr::MulInt,
        9 => Instr::DivInt,
        10 => Instr::ModInt,
        11 => Instr::Eq,
        12 => Instr::Neq,
        13 => Instr::LtInt,
        14 => Instr::LteInt,
        15 => Instr::GtInt,
        16 => Instr::GteInt,
        17 => Instr::AndBool,
        18 => Instr::OrBool,
        19 => Instr::Jump(rd.read_u32()? as usize),
        20 => Instr::JumpIfFalse(rd.read_u32()? as usize),
        21 => Instr::JumpIfTrue(rd.read_u32()? as usize),
        22 => Instr::Call {
            name: rd.read_str()?,
            argc: rd.read_u32()? as usize,
        },
        23 => Instr::CallBuiltin {
            package: rd.read_str()?,
            name: rd.read_str()?,
            argc: rd.read_u32()? as usize,
        },
        24 => Instr::Return,
        t => return Err(format!("Unknown instruction tag {t}")),
    })
}

fn fmt_value(v: &Value) -> String {
    match v {
        Value::Int(i) => format!("Int({i})"),
        Value::Float(n) => format!("Float({n})"),
        Value::Bool(b) => format!("Bool({b})"),
        Value::String(s) => format!("String({s:?})"),
        Value::Unit => "Unit".to_string(),
    }
}

fn fmt_instr(i: &Instr) -> String {
    match i {
        Instr::LoadConst(v) => format!("LoadConst {}", fmt_value(v)),
        Instr::LoadLocal(s) => format!("LoadLocal {s}"),
        Instr::StoreLocal(s) => format!("StoreLocal {s}"),
        Instr::Pop => "Pop".to_string(),
        Instr::NegInt => "NegInt".to_string(),
        Instr::NotBool => "NotBool".to_string(),
        Instr::Add => "Add".to_string(),
        Instr::SubInt => "SubInt".to_string(),
        Instr::MulInt => "MulInt".to_string(),
        Instr::DivInt => "DivInt".to_string(),
        Instr::ModInt => "ModInt".to_string(),
        Instr::Eq => "Eq".to_string(),
        Instr::Neq => "Neq".to_string(),
        Instr::LtInt => "LtInt".to_string(),
        Instr::LteInt => "LteInt".to_string(),
        Instr::GtInt => "GtInt".to_string(),
        Instr::GteInt => "GteInt".to_string(),
        Instr::AndBool => "AndBool".to_string(),
        Instr::OrBool => "OrBool".to_string(),
        Instr::Jump(t) => format!("Jump {t}"),
        Instr::JumpIfFalse(t) => format!("JumpIfFalse {t}"),
        Instr::JumpIfTrue(t) => format!("JumpIfTrue {t}"),
        Instr::Call { name, argc } => format!("Call {name} argc={argc}"),
        Instr::CallBuiltin {
            package,
            name,
            argc,
        } => format!("CallBuiltin {package}.{name} argc={argc}"),
        Instr::Return => "Return".to_string(),
    }
}
