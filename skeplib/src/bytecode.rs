use std::collections::HashMap;

use crate::ast::{AssignTarget, BinaryOp, Expr, Program, Stmt, UnaryOp};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::parser::Parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    String(String),
    Unit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    Call { name: String, argc: usize },
    CallBuiltin {
        package: String,
        name: String,
        argc: usize,
    },
    Return,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FunctionChunk {
    pub name: String,
    pub code: Vec<Instr>,
    pub locals_count: usize,
    pub param_count: usize,
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
            let chunk = self.compile_function(func);
            module.functions.insert(func.name.clone(), chunk);
        }
        module
    }

    fn compile_function(&mut self, func: &crate::ast::FnDecl) -> FunctionChunk {
        let mut ctx = FnCtx::default();
        let mut code = Vec::new();

        for param in &func.params {
            ctx.alloc_local(param.name.clone());
        }

        for stmt in &func.body {
            self.compile_stmt(stmt, &mut ctx, &mut code);
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
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => {
                self.compile_expr(cond, ctx, code);
                let jmp_false_at = code.len();
                code.push(Instr::JumpIfFalse(usize::MAX));

                for s in then_body {
                    self.compile_stmt(s, ctx, code);
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
                        self.compile_stmt(s, ctx, code);
                    }

                    let end = code.len();
                    code[jmp_end_at] = Instr::Jump(end);
                }
            }
            Stmt::While { cond, body } => {
                let loop_start = code.len();
                self.compile_expr(cond, ctx, code);
                let jmp_false_at = code.len();
                code.push(Instr::JumpIfFalse(usize::MAX));

                for s in body {
                    self.compile_stmt(s, ctx, code);
                }

                code.push(Instr::Jump(loop_start));
                let loop_end = code.len();
                code[jmp_false_at] = Instr::JumpIfFalse(loop_end);
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr, ctx: &mut FnCtx, code: &mut Vec<Instr>) {
        match expr {
            Expr::IntLit(v) => code.push(Instr::LoadConst(Value::Int(*v))),
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
                UnaryOp::Not => {
                    self.compile_expr(expr, ctx, code);
                    code.push(Instr::NotBool);
                }
            },
            Expr::Binary { left, op, right } => {
                self.compile_expr(left, ctx, code);
                self.compile_expr(right, ctx, code);
                match op {
                    BinaryOp::Add => code.push(Instr::Add),
                    BinaryOp::Sub => code.push(Instr::SubInt),
                    BinaryOp::Mul => code.push(Instr::MulInt),
                    BinaryOp::Div => code.push(Instr::DivInt),
                    BinaryOp::EqEq => code.push(Instr::Eq),
                    BinaryOp::Neq => code.push(Instr::Neq),
                    BinaryOp::Lt => code.push(Instr::LtInt),
                    BinaryOp::Lte => code.push(Instr::LteInt),
                    BinaryOp::Gt => code.push(Instr::GtInt),
                    BinaryOp::Gte => code.push(Instr::GteInt),
                    BinaryOp::AndAnd => code.push(Instr::AndBool),
                    BinaryOp::OrOr => code.push(Instr::OrBool),
                }
            }
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
                        self.error("Only direct function calls are supported in bytecode v0 slice".to_string());
                        return;
                    }
                };
                for arg in args {
                    self.compile_expr(arg, ctx, code);
                }
                code.push(Instr::Call { name, argc: args.len() });
            }
            Expr::Group(inner) => self.compile_expr(inner, ctx, code),
            Expr::Path(_) => {
                self.error("Path expression value is not supported in bytecode v0 compiler slice".to_string());
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
