use std::collections::HashMap;

use crate::ast::{AssignTarget, BinaryOp, Expr, Program, Stmt, UnaryOp};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::parser::Parser;

use super::{BytecodeModule, FunctionChunk, Instr, Value};

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
            Stmt::Assign { target, value } => match target {
                AssignTarget::Ident(name) => {
                    self.compile_expr(value, ctx, code);
                    if let Some(slot) = ctx.lookup(name) {
                        code.push(Instr::StoreLocal(slot));
                    } else {
                        self.error(format!("Unknown local `{name}` in assignment"));
                    }
                }
                AssignTarget::Path(parts) => {
                    self.compile_expr(value, ctx, code);
                    self.error(format!(
                        "Path assignment not supported in bytecode v0: {}",
                        parts.join(".")
                    ));
                }
                AssignTarget::Index { base, index } => {
                    if let Some((root, indices)) = Self::flatten_index_target(base, index) {
                        if let Some(slot) = ctx.lookup(&root) {
                            code.push(Instr::LoadLocal(slot));
                            for idx in &indices {
                                self.compile_expr(idx, ctx, code);
                            }
                            self.compile_expr(value, ctx, code);
                            if indices.len() == 1 {
                                code.push(Instr::ArraySet);
                            } else {
                                code.push(Instr::ArraySetChain(indices.len()));
                            }
                            code.push(Instr::StoreLocal(slot));
                        } else {
                            self.error(format!("Unknown local `{root}` in index assignment"));
                        }
                    } else {
                        self.error("Unsupported index assignment target".to_string());
                    }
                }
                AssignTarget::Field { .. } => {
                    self.compile_expr(value, ctx, code);
                    self.error("Field assignment is not supported in bytecode v0".to_string());
                }
            },
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
                    continue_target: loop_start,
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
            Stmt::For {
                init,
                cond,
                step,
                body,
            } => {
                if let Some(init) = init {
                    self.compile_stmt(init, ctx, loops, code);
                }

                let cond_start = code.len();
                if let Some(cond) = cond {
                    self.compile_expr(cond, ctx, code);
                } else {
                    code.push(Instr::LoadConst(Value::Bool(true)));
                }
                let jmp_false_at = code.len();
                code.push(Instr::JumpIfFalse(usize::MAX));

                // Jump to body first; step block is placed before body so `continue`
                // can always target a known address.
                let jmp_body_at = code.len();
                code.push(Instr::Jump(usize::MAX));
                let step_start = code.len();
                loops.push(LoopCtx {
                    continue_target: step_start,
                    break_jumps: Vec::new(),
                });

                if let Some(step) = step {
                    self.compile_stmt(step, ctx, loops, code);
                }
                code.push(Instr::Jump(cond_start));
                let body_start = code.len();
                code[jmp_body_at] = Instr::Jump(body_start);

                for s in body {
                    self.compile_stmt(s, ctx, loops, code);
                }
                code.push(Instr::Jump(step_start));

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
                    self.error("`break` used outside a loop".to_string());
                }
            }
            Stmt::Continue => {
                if let Some(lp) = loops.last() {
                    code.push(Instr::Jump(lp.continue_target));
                } else {
                    self.error("`continue` used outside a loop".to_string());
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
            Expr::ArrayLit(items) => {
                for item in items {
                    self.compile_expr(item, ctx, code);
                }
                code.push(Instr::MakeArray(items.len()));
            }
            Expr::ArrayRepeat { value, size } => {
                self.compile_expr(value, ctx, code);
                code.push(Instr::MakeArrayRepeat(*size));
            }
            Expr::Index { base, index } => {
                self.compile_expr(base, ctx, code);
                self.compile_expr(index, ctx, code);
                code.push(Instr::ArrayGet);
            }
            Expr::Field { .. } => {
                self.error(
                    "Field access is not supported in bytecode v0 compiler slice".to_string(),
                );
            }
            Expr::StructLit { .. } => {
                self.error(
                    "Struct literals are not supported in bytecode v0 compiler slice".to_string(),
                );
            }
        }
    }

    fn error(&mut self, message: String) {
        self.diags.error(message, Span::default());
    }

    fn flatten_index_target<'a>(
        base: &'a Expr,
        index: &'a Expr,
    ) -> Option<(String, Vec<&'a Expr>)> {
        let mut indices = vec![index];
        let mut cur = base;
        loop {
            match cur {
                Expr::Ident(name) => {
                    indices.reverse();
                    return Some((name.clone(), indices));
                }
                Expr::Index { base, index } => {
                    indices.push(index);
                    cur = base;
                }
                _ => return None,
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
struct LoopCtx {
    continue_target: usize,
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
