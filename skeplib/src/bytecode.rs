use std::collections::HashMap;

use crate::ast::{AssignTarget, BinaryOp, Expr, Program, Stmt, UnaryOp};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::parser::Parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Unit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instr {
    LoadConst(Value),
    LoadLocal(usize),
    StoreLocal(usize),
    Pop,
    NegInt,
    AddInt,
    SubInt,
    MulInt,
    DivInt,
    Return,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FunctionChunk {
    pub name: String,
    pub code: Vec<Instr>,
    pub locals_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BytecodeModule {
    pub functions: HashMap<String, FunctionChunk>,
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
            let chunk = self.compile_function(func.name.as_str(), &func.body);
            module.functions.insert(func.name.clone(), chunk);
        }
        module
    }

    fn compile_function(&mut self, name: &str, body: &[Stmt]) -> FunctionChunk {
        let mut ctx = FnCtx::default();
        let mut code = Vec::new();

        for stmt in body {
            self.compile_stmt(stmt, &mut ctx, &mut code);
        }

        if !matches!(code.last(), Some(Instr::Return)) {
            code.push(Instr::LoadConst(Value::Unit));
            code.push(Instr::Return);
        }

        FunctionChunk {
            name: name.to_string(),
            code,
            locals_count: ctx.next_local,
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt, ctx: &mut FnCtx, code: &mut Vec<Instr>) {
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
            Stmt::If { .. } | Stmt::While { .. } => {
                self.error("if/while not supported in bytecode v0 compiler slice".to_string());
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr, ctx: &mut FnCtx, code: &mut Vec<Instr>) {
        match expr {
            Expr::IntLit(v) => code.push(Instr::LoadConst(Value::Int(*v))),
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
                UnaryOp::Not => {
                    self.error("Unary ! not supported in bytecode v0 compiler slice".to_string());
                }
            },
            Expr::Binary { left, op, right } => {
                self.compile_expr(left, ctx, code);
                self.compile_expr(right, ctx, code);
                match op {
                    BinaryOp::Add => code.push(Instr::AddInt),
                    BinaryOp::Sub => code.push(Instr::SubInt),
                    BinaryOp::Mul => code.push(Instr::MulInt),
                    BinaryOp::Div => code.push(Instr::DivInt),
                    _ => self.error(format!("Operator {:?} not supported in bytecode v0 compiler slice", op)),
                }
            }
            Expr::Group(inner) => self.compile_expr(inner, ctx, code),
            Expr::Call { .. }
            | Expr::BoolLit(_)
            | Expr::StringLit(_)
            | Expr::Path(_) => {
                self.error("Expression kind not supported in bytecode v0 compiler slice".to_string());
            }
        }
    }

    fn error(&mut self, message: String) {
        self.diags.error(message, Span::default());
    }
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
