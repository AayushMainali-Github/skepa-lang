use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::ast::{
    AssignTarget, BinaryOp, Expr, MatchLiteral, MatchPattern, Program, Stmt, TypeName, UnaryOp,
};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::parser::Parser;
use crate::resolver::{ModuleGraph, ModuleId, ResolveError, build_export_maps, resolve_project};

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
        let mut module = module;
        rewrite_direct_calls_to_indexes(&mut module);
        Ok(module)
    } else {
        Err(diags)
    }
}

pub fn compile_project_entry(entry: &Path) -> Result<BytecodeModule, Vec<ResolveError>> {
    let graph = resolve_project(entry)?;
    compile_project_graph(&graph, entry).map_err(|e| {
        vec![ResolveError::new(
            crate::resolver::ResolveErrorKind::Codegen,
            e,
            Some(entry.to_path_buf()),
        )]
    })
}

#[derive(Default)]
struct Compiler {
    diags: DiagnosticBag,
    function_names: HashSet<String>,
    global_slots: HashMap<String, usize>,
    direct_import_calls: HashMap<String, String>,
    module_namespaces: HashMap<String, Vec<String>>,
    lifted_functions: Vec<FunctionChunk>,
    fn_lit_counter: usize,
    module_id: Option<String>,
    local_fn_qualified: HashMap<String, String>,
    local_struct_runtime: HashMap<String, String>,
    imported_struct_runtime: HashMap<String, String>,
    namespace_call_targets: HashMap<String, String>,
}

impl Compiler {
    fn mangle_method_name(target: &str, method: &str) -> String {
        format!("__impl_{}__{}", target, method)
    }

    fn globals_init_name(&self) -> String {
        match &self.module_id {
            Some(id) => format!("__globals_init::{id}"),
            None => "__globals_init".to_string(),
        }
    }

    fn qualify_local_fn_name(&self, local: &str) -> String {
        match &self.module_id {
            Some(id) => format!("{id}::{local}"),
            None => local.to_string(),
        }
    }

    fn resolve_struct_runtime_name(&self, name: &str) -> String {
        if let Some(v) = self.local_struct_runtime.get(name) {
            return v.clone();
        }
        if let Some(v) = self.imported_struct_runtime.get(name) {
            return v.clone();
        }
        name.to_string()
    }

    fn expr_to_parts(expr: &Expr) -> Option<Vec<String>> {
        match expr {
            Expr::Ident(name) => Some(vec![name.clone()]),
            Expr::Path(parts) => Some(parts.clone()),
            Expr::Field { base, field } => {
                let mut parts = Self::expr_to_parts(base)?;
                parts.push(field.clone());
                Some(parts)
            }
            _ => None,
        }
    }

    fn compile_program(&mut self, program: &Program) -> BytecodeModule {
        self.function_names.clear();
        self.global_slots.clear();
        self.local_fn_qualified.clear();
        if self.module_id.is_none() {
            self.direct_import_calls.clear();
            self.module_namespaces.clear();
            self.local_struct_runtime.clear();
            self.imported_struct_runtime.clear();
            self.namespace_call_targets.clear();
        }
        self.lifted_functions.clear();
        self.fn_lit_counter = 0;
        const GLOBALS_INIT_FN: &str = "__globals_init";
        if self.module_id.is_none() && program.functions.iter().any(|f| f.name == GLOBALS_INIT_FN) {
            self.error(format!(
                "`{GLOBALS_INIT_FN}` is a reserved function name used by the compiler"
            ));
        }
        for func in &program.functions {
            let q = self.qualify_local_fn_name(&func.name);
            self.local_fn_qualified.insert(func.name.clone(), q.clone());
            self.function_names.insert(q);
        }
        for imp in &program.impls {
            for method in &imp.methods {
                let target_name = self.resolve_struct_runtime_name(&imp.target);
                self.function_names
                    .insert(Self::mangle_method_name(&target_name, &method.name));
            }
        }
        for s in &program.structs {
            let runtime = match &self.module_id {
                Some(id) => format!("{id}::{}", s.name),
                None => s.name.clone(),
            };
            self.local_struct_runtime.insert(s.name.clone(), runtime);
        }
        for (idx, g) in program.globals.iter().enumerate() {
            self.global_slots.insert(g.name.clone(), idx);
        }
        if self.module_id.is_none() {
            for imp in &program.imports {
                match imp {
                    crate::ast::ImportDecl::ImportModule { path, alias } => {
                        let ns = alias
                            .clone()
                            .unwrap_or_else(|| path.first().cloned().unwrap_or_default());
                        if !ns.is_empty() {
                            let mapped = if alias.is_some() {
                                path.clone()
                            } else {
                                vec![path.first().cloned().unwrap_or_default()]
                            };
                            self.module_namespaces.insert(ns, mapped);
                        }
                    }
                    crate::ast::ImportDecl::ImportFrom {
                        path,
                        wildcard: _,
                        items,
                    } => {
                        let prefix = path.join(".");
                        for item in items {
                            let local = item.alias.clone().unwrap_or_else(|| item.name.clone());
                            self.direct_import_calls
                                .insert(local, format!("{prefix}.{}", item.name));
                        }
                    }
                }
            }
        }
        let mut module = BytecodeModule::default();
        if !program.globals.is_empty() {
            let init = self.compile_globals_init(program);
            module.functions.insert(init.name.clone(), init);
        }
        for func in &program.functions {
            let chunk = self.compile_function(func);
            module.functions.insert(chunk.name.clone(), chunk);
        }
        for imp in &program.impls {
            for method in &imp.methods {
                let target_name = self.resolve_struct_runtime_name(&imp.target);
                let mangled = Self::mangle_method_name(&target_name, &method.name);
                let chunk = self.compile_method(&mangled, method);
                module.functions.insert(mangled, chunk);
            }
        }
        for chunk in self.lifted_functions.drain(..) {
            module.functions.insert(chunk.name.clone(), chunk);
        }
        module
    }

    fn compile_globals_init(&mut self, program: &Program) -> FunctionChunk {
        let mut code = Vec::new();
        let mut ctx = FnCtx::default();
        for g in &program.globals {
            self.compile_expr(&g.value, &mut ctx, &mut code);
            if let Some(slot) = self.global_slots.get(&g.name).copied() {
                code.push(Instr::StoreGlobal(slot));
            }
        }
        code.push(Instr::LoadConst(Value::Unit));
        code.push(Instr::Return);
        FunctionChunk {
            name: self.globals_init_name(),
            code,
            locals_count: program.globals.len(),
            param_count: 0,
        }
    }

    fn compile_fn_lit(&mut self, params: &[crate::ast::Param], body: &[Stmt]) -> String {
        self.fn_lit_counter += 1;
        let name = format!("__fn_lit_{}", self.fn_lit_counter);
        self.function_names.insert(name.clone());

        let mut ctx = FnCtx::default();
        let mut loops: Vec<LoopCtx> = Vec::new();
        let mut code = Vec::new();

        for param in params {
            if let TypeName::Named(type_name) = &param.ty {
                ctx.alloc_local_with_named_type(param.name.clone(), type_name.clone());
            } else {
                ctx.alloc_local(param.name.clone());
            }
        }

        for stmt in body {
            self.compile_stmt(stmt, &mut ctx, &mut loops, &mut code);
        }

        if !matches!(code.last(), Some(Instr::Return)) {
            code.push(Instr::LoadConst(Value::Unit));
            code.push(Instr::Return);
        }

        self.lifted_functions.push(FunctionChunk {
            name: name.clone(),
            code,
            locals_count: ctx.next_local,
            param_count: params.len(),
        });
        name
    }

    fn compile_function(&mut self, func: &crate::ast::FnDecl) -> FunctionChunk {
        let mut ctx = FnCtx::default();
        let mut loops: Vec<LoopCtx> = Vec::new();
        let mut code = Vec::new();

        for param in &func.params {
            if let TypeName::Named(type_name) = &param.ty {
                ctx.alloc_local_with_named_type(
                    param.name.clone(),
                    self.resolve_struct_runtime_name(type_name),
                );
            } else {
                ctx.alloc_local(param.name.clone());
            }
        }

        for stmt in &func.body {
            self.compile_stmt(stmt, &mut ctx, &mut loops, &mut code);
        }

        if !matches!(code.last(), Some(Instr::Return)) {
            code.push(Instr::LoadConst(Value::Unit));
            code.push(Instr::Return);
        }

        FunctionChunk {
            name: self.qualify_local_fn_name(&func.name),
            code,
            locals_count: ctx.next_local,
            param_count: func.params.len(),
        }
    }

    fn compile_method(&mut self, name: &str, method: &crate::ast::MethodDecl) -> FunctionChunk {
        let mut ctx = FnCtx::default();
        let mut loops: Vec<LoopCtx> = Vec::new();
        let mut code = Vec::new();

        for param in &method.params {
            if let TypeName::Named(type_name) = &param.ty {
                ctx.alloc_local_with_named_type(
                    param.name.clone(),
                    self.resolve_struct_runtime_name(type_name),
                );
            } else {
                ctx.alloc_local(param.name.clone());
            }
        }

        for stmt in &method.body {
            self.compile_stmt(stmt, &mut ctx, &mut loops, &mut code);
        }

        if !matches!(code.last(), Some(Instr::Return)) {
            code.push(Instr::LoadConst(Value::Unit));
            code.push(Instr::Return);
        }

        FunctionChunk {
            name: name.to_string(),
            code,
            locals_count: ctx.next_local,
            param_count: method.params.len(),
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
            Stmt::Let { name, ty, value } => {
                self.compile_expr(value, ctx, code);
                let explicit_named = match ty {
                    Some(TypeName::Named(type_name)) => {
                        Some(self.resolve_struct_runtime_name(type_name))
                    }
                    _ => None,
                };
                let inferred_named = Self::infer_expr_named_type(value, ctx);
                let slot = if let Some(type_name) = explicit_named.or(inferred_named) {
                    ctx.alloc_local_with_named_type(name.clone(), type_name)
                } else {
                    ctx.alloc_local(name.clone())
                };
                code.push(Instr::StoreLocal(slot));
            }
            Stmt::Assign { target, value } => match target {
                AssignTarget::Ident(name) => {
                    self.compile_expr(value, ctx, code);
                    if let Some(slot) = ctx.lookup(name) {
                        code.push(Instr::StoreLocal(slot));
                    } else if let Some(slot) = self.global_slots.get(name).copied() {
                        code.push(Instr::StoreGlobal(slot));
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
                    if let Some((root, fields)) = Self::flatten_field_target(target) {
                        if let Some(slot) = ctx.lookup(&root) {
                            code.push(Instr::LoadLocal(slot));
                            self.compile_expr(value, ctx, code);
                            code.push(Instr::StructSetPath(fields));
                            code.push(Instr::StoreLocal(slot));
                        } else {
                            self.error("Path assignment not supported in bytecode v0".to_string());
                        }
                    } else {
                        self.compile_expr(value, ctx, code);
                        self.error(
                            "Unsupported field assignment target in bytecode compiler".to_string(),
                        );
                    }
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
            Stmt::Match { expr, arms } => {
                self.compile_expr(expr, ctx, code);
                let match_slot = ctx.alloc_anonymous_local();
                code.push(Instr::StoreLocal(match_slot));

                let mut end_jumps = Vec::new();
                for arm in arms {
                    self.compile_match_pattern_condition(&arm.pattern, match_slot, code);
                    let jmp_false_at = code.len();
                    code.push(Instr::JumpIfFalse(usize::MAX));

                    for s in &arm.body {
                        self.compile_stmt(s, ctx, loops, code);
                    }

                    let jmp_end_at = code.len();
                    code.push(Instr::Jump(usize::MAX));
                    end_jumps.push(jmp_end_at);

                    let next_arm = code.len();
                    code[jmp_false_at] = Instr::JumpIfFalse(next_arm);
                }

                let end = code.len();
                for at in end_jumps {
                    code[at] = Instr::Jump(end);
                }
            }
        }
    }

    fn compile_match_pattern_condition(
        &mut self,
        pattern: &MatchPattern,
        match_slot: usize,
        code: &mut Vec<Instr>,
    ) {
        match pattern {
            MatchPattern::Wildcard => code.push(Instr::LoadConst(Value::Bool(true))),
            MatchPattern::Literal(lit) => {
                code.push(Instr::LoadLocal(match_slot));
                self.compile_match_literal(lit, code);
                code.push(Instr::Eq);
            }
            MatchPattern::Or(parts) => {
                let mut iter = parts.iter();
                if let Some(first) = iter.next() {
                    self.compile_match_pattern_condition(first, match_slot, code);
                    for part in iter {
                        self.compile_match_pattern_condition(part, match_slot, code);
                        code.push(Instr::OrBool);
                    }
                } else {
                    code.push(Instr::LoadConst(Value::Bool(false)));
                }
            }
        }
    }

    fn compile_match_literal(&mut self, lit: &MatchLiteral, code: &mut Vec<Instr>) {
        match lit {
            MatchLiteral::Int(v) => code.push(Instr::LoadConst(Value::Int(*v))),
            MatchLiteral::Bool(v) => code.push(Instr::LoadConst(Value::Bool(*v))),
            MatchLiteral::String(v) => code.push(Instr::LoadConst(Value::String(v.clone()))),
            MatchLiteral::Float(v) => match v.parse::<f64>() {
                Ok(n) => code.push(Instr::LoadConst(Value::Float(n))),
                Err(_) => {
                    self.error(format!("Invalid float literal in match pattern `{v}`"));
                    code.push(Instr::LoadConst(Value::Float(0.0)));
                }
            },
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
                } else if let Some(slot) = self.global_slots.get(name).copied() {
                    code.push(Instr::LoadGlobal(slot));
                } else if let Some(target) = self.direct_import_calls.get(name).cloned() {
                    code.push(Instr::LoadConst(Value::Function(target)));
                } else if let Some(target) = self.local_fn_qualified.get(name).cloned() {
                    code.push(Instr::LoadConst(Value::Function(target)));
                } else if self.function_names.contains(name) {
                    code.push(Instr::LoadConst(Value::Function(name.clone())));
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
            Expr::Call { callee, args } => match &**callee {
                Expr::Ident(name) => {
                    if ctx.lookup(name).is_some() {
                        self.compile_expr(callee, ctx, code);
                        for arg in args {
                            self.compile_expr(arg, ctx, code);
                        }
                        code.push(Instr::CallValue { argc: args.len() });
                    } else if let Some(target) = self.direct_import_calls.get(name).cloned() {
                        for arg in args {
                            self.compile_expr(arg, ctx, code);
                        }
                        code.push(Instr::Call {
                            name: target,
                            argc: args.len(),
                        });
                    } else if let Some(target) = self.local_fn_qualified.get(name).cloned() {
                        for arg in args {
                            self.compile_expr(arg, ctx, code);
                        }
                        code.push(Instr::Call {
                            name: target,
                            argc: args.len(),
                        });
                    } else {
                        for arg in args {
                            self.compile_expr(arg, ctx, code);
                        }
                        code.push(Instr::Call {
                            name: name.clone(),
                            argc: args.len(),
                        });
                    }
                }
                Expr::Field { base, field } => {
                    if let Some(parts) = Self::expr_to_parts(callee)
                        && parts.len() == 2
                        && matches!(&**base, Expr::Ident(pkg) if ctx.lookup(pkg).is_none())
                    {
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
                    if let Some(parts) = Self::expr_to_parts(callee)
                        && let Some(target) = self.resolve_qualified_import_call(&parts)
                    {
                        for arg in args {
                            self.compile_expr(arg, ctx, code);
                        }
                        code.push(Instr::Call {
                            name: target,
                            argc: args.len(),
                        });
                        return;
                    }
                    if let Some(parts) = Self::expr_to_parts(callee)
                        && parts.len() > 2
                    {
                        self.error(
                            "Only `package.function(...)` builtins are supported".to_string(),
                        );
                        return;
                    }

                    self.compile_expr(base, ctx, code);
                    for arg in args {
                        self.compile_expr(arg, ctx, code);
                    }
                    code.push(Instr::CallMethod {
                        name: field.clone(),
                        argc: args.len(),
                    });
                }
                _ => {
                    self.compile_expr(callee, ctx, code);
                    for arg in args {
                        self.compile_expr(arg, ctx, code);
                    }
                    code.push(Instr::CallValue { argc: args.len() });
                }
            },
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
                if let Some((base, fields)) = Self::flatten_field_expr(expr) {
                    self.compile_expr(base, ctx, code);
                    for field in fields {
                        code.push(Instr::StructGet(field));
                    }
                } else {
                    self.error("Unsupported field access shape in bytecode compiler".to_string());
                }
            }
            Expr::StructLit { name, fields } => {
                for (_, value) in fields {
                    self.compile_expr(value, ctx, code);
                }
                code.push(Instr::MakeStruct {
                    name: self.resolve_struct_runtime_name(name),
                    fields: fields.iter().map(|(k, _)| k.clone()).collect(),
                });
            }
            Expr::FnLit { params, body, .. } => {
                let fn_name = self.compile_fn_lit(params, body);
                code.push(Instr::LoadConst(Value::Function(fn_name)));
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

    fn flatten_field_expr(expr: &Expr) -> Option<(&Expr, Vec<String>)> {
        let mut fields = Vec::new();
        let mut cur = expr;
        loop {
            match cur {
                Expr::Field { base, field } => {
                    fields.push(field.clone());
                    cur = base;
                }
                _ => {
                    fields.reverse();
                    return Some((cur, fields));
                }
            }
        }
    }

    fn flatten_field_target(target: &AssignTarget) -> Option<(String, Vec<String>)> {
        let AssignTarget::Field { base, field } = target else {
            return None;
        };
        let mut fields = vec![field.clone()];
        let mut cur = base.as_ref();
        loop {
            match cur {
                Expr::Field { base, field } => {
                    fields.push(field.clone());
                    cur = base;
                }
                Expr::Ident(name) => {
                    fields.reverse();
                    return Some((name.clone(), fields));
                }
                _ => return None,
            }
        }
    }

    fn infer_expr_named_type(expr: &Expr, ctx: &FnCtx) -> Option<String> {
        match expr {
            Expr::Ident(name) => ctx.named_type(name),
            Expr::StructLit { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    fn resolve_qualified_import_call(&self, parts: &[String]) -> Option<String> {
        let key = parts.join(".");
        if let Some(target) = self.namespace_call_targets.get(&key) {
            return Some(target.clone());
        }
        let prefix = self.module_namespaces.get(parts.first()?)?.clone();
        let mut full = prefix;
        full.extend_from_slice(&parts[1..]);
        Some(full.join("."))
    }
}

fn compile_project_graph(graph: &ModuleGraph, entry: &Path) -> Result<BytecodeModule, String> {
    let mut programs = HashMap::<ModuleId, Program>::new();
    for (id, unit) in &graph.modules {
        let (program, diags) = Parser::parse_source(&unit.source);
        if !diags.is_empty() {
            return Err(format!("Parse failed in module `{id}`"));
        }
        programs.insert(id.clone(), program);
    }
    let export_maps = build_export_maps(graph)
        .map_err(|errs| format!("Export validation failed: {}", errs[0].message))?;

    let entry_id = graph
        .modules
        .iter()
        .find_map(|(id, unit)| {
            if unit.path == entry {
                Some(id.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| "Entry module id not found in graph".to_string())?;

    let mut out = BytecodeModule::default();
    let mut init_names = Vec::new();
    let mut ids = programs.keys().cloned().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        let program = programs
            .get(&id)
            .ok_or_else(|| format!("Missing parsed program for module `{id}`"))?;
        let mut c = Compiler {
            module_id: Some(id.clone()),
            ..Compiler::default()
        };
        c.local_struct_runtime = program
            .structs
            .iter()
            .map(|s| (s.name.clone(), format!("{id}::{}", s.name)))
            .collect();

        for imp in &program.imports {
            match imp {
                crate::ast::ImportDecl::ImportFrom {
                    path,
                    wildcard,
                    items,
                } => {
                    let target = path.join(".");
                    let Some(exports) = export_maps.get(&target) else {
                        continue;
                    };
                    if *wildcard {
                        for (name, sym) in exports {
                            match sym.kind {
                                crate::resolver::SymbolKind::Fn => {
                                    c.direct_import_calls.insert(
                                        name.clone(),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                                crate::resolver::SymbolKind::Struct => {
                                    c.imported_struct_runtime.insert(
                                        name.clone(),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                                crate::resolver::SymbolKind::GlobalLet
                                | crate::resolver::SymbolKind::Namespace => {}
                            }
                        }
                    } else {
                        for item in items {
                            let local = item.alias.clone().unwrap_or_else(|| item.name.clone());
                            if let Some(sym) = exports.get(&item.name) {
                                match sym.kind {
                                    crate::resolver::SymbolKind::Fn => {
                                        c.direct_import_calls.insert(
                                            local,
                                            format!("{}::{}", sym.module_id, sym.local_name),
                                        );
                                    }
                                    crate::resolver::SymbolKind::Struct => {
                                        c.imported_struct_runtime.insert(
                                            local,
                                            format!("{}::{}", sym.module_id, sym.local_name),
                                        );
                                    }
                                    crate::resolver::SymbolKind::GlobalLet
                                    | crate::resolver::SymbolKind::Namespace => {}
                                }
                            }
                        }
                    }
                }
                crate::ast::ImportDecl::ImportModule { path, alias } => {
                    let ns = alias
                        .clone()
                        .unwrap_or_else(|| path.first().cloned().unwrap_or_default());
                    if ns.is_empty() {
                        continue;
                    }
                    let prefix = if alias.is_some() {
                        path.clone()
                    } else {
                        vec![path.first().cloned().unwrap_or_default()]
                    };
                    c.module_namespaces.insert(ns.clone(), prefix);
                    let target_prefix = path.join(".");
                    let mut exporting = export_maps
                        .keys()
                        .filter(|m| {
                            *m == &target_prefix || m.starts_with(&(target_prefix.clone() + "."))
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    exporting.sort();
                    for mid in exporting {
                        if let Some(exports) = export_maps.get(&mid) {
                            for (ename, sym) in exports {
                                if sym.kind == crate::resolver::SymbolKind::Fn {
                                    c.namespace_call_targets.insert(
                                        format!("{mid}.{ename}"),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                                if sym.kind == crate::resolver::SymbolKind::Struct {
                                    c.imported_struct_runtime.insert(
                                        format!("{mid}.{ename}"),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        let m = c.compile_program(program);
        let init = c.globals_init_name();
        if m.functions.contains_key(&init) {
            init_names.push(init);
        }
        let mut names = m.functions.keys().cloned().collect::<Vec<_>>();
        names.sort();
        for n in names {
            if let Some(chunk) = m.functions.get(&n).cloned()
                && out.functions.insert(n.clone(), chunk).is_some()
            {
                return Err(format!("Duplicate linked function symbol `{n}`"));
            }
        }
    }

    if !init_names.is_empty() {
        init_names.sort();
        let mut code = Vec::new();
        for n in init_names {
            code.push(Instr::Call { name: n, argc: 0 });
            code.push(Instr::Pop);
        }
        code.push(Instr::LoadConst(Value::Unit));
        code.push(Instr::Return);
        out.functions.insert(
            "__globals_init".to_string(),
            FunctionChunk {
                name: "__globals_init".to_string(),
                code,
                locals_count: 0,
                param_count: 0,
            },
        );
    }

    out.functions.insert(
        "main".to_string(),
        FunctionChunk {
            name: "main".to_string(),
            code: vec![
                Instr::Call {
                    name: format!("{entry_id}::main"),
                    argc: 0,
                },
                Instr::Return,
            ],
            locals_count: 0,
            param_count: 0,
        },
    );

    rewrite_direct_calls_to_indexes(&mut out);
    Ok(out)
}

#[derive(Debug, Clone, Default)]
struct LoopCtx {
    continue_target: usize,
    break_jumps: Vec<usize>,
}

#[derive(Default)]
struct FnCtx {
    locals: HashMap<String, usize>,
    local_named_types: HashMap<String, String>,
    next_local: usize,
}

impl FnCtx {
    fn alloc_local(&mut self, name: String) -> usize {
        let slot = self.next_local;
        self.next_local += 1;
        self.locals.insert(name, slot);
        slot
    }

    fn alloc_local_with_named_type(&mut self, name: String, type_name: String) -> usize {
        let slot = self.alloc_local(name.clone());
        self.local_named_types.insert(name, type_name);
        slot
    }

    fn alloc_anonymous_local(&mut self) -> usize {
        let slot = self.next_local;
        self.next_local += 1;
        slot
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.locals.get(name).copied()
    }

    fn named_type(&self, name: &str) -> Option<String> {
        self.local_named_types.get(name).cloned()
    }
}

fn rewrite_direct_calls_to_indexes(module: &mut BytecodeModule) {
    let mut names = module.functions.keys().cloned().collect::<Vec<_>>();
    names.sort();
    let by_name = names
        .into_iter()
        .enumerate()
        .map(|(idx, n)| (n, idx))
        .collect::<HashMap<_, _>>();

    for chunk in module.functions.values_mut() {
        for instr in &mut chunk.code {
            let new_instr = match instr {
                Instr::Call { name, argc } => by_name
                    .get(name)
                    .copied()
                    .map(|idx| Instr::CallIdx { idx, argc: *argc }),
                _ => None,
            };
            if let Some(new_instr) = new_instr {
                *instr = new_instr;
            }
        }
    }
}
