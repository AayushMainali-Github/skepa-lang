use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

mod context;
mod expr;
mod peephole;
mod project;
mod specialize;
mod stmt;

use crate::ast::{AssignTarget, BinaryOp, Expr, Program, Stmt, TypeName, UnaryOp};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::parser::Parser;
use crate::resolver::{ModuleGraph, ResolveError, resolve_project};

use self::context::{Compiler, FnCtx, InlinableFunction, InlinableMethod, LoopCtx, StructLayout};
use self::peephole::{
    peephole_optimize_module, rewrite_direct_calls_to_indexes, rewrite_function_values_to_indexes,
    rewrite_trivial_direct_calls,
};
use super::{BytecodeModule, FunctionChunk, Instr, IntLocalConstOp, StructShape, Value};

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
        peephole_optimize_module(&mut module);
        rewrite_direct_calls_to_indexes(&mut module);
        rewrite_trivial_direct_calls(&mut module);
        rewrite_function_values_to_indexes(&mut module);
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

pub fn compile_project_graph(graph: &ModuleGraph, entry: &Path) -> Result<BytecodeModule, String> {
    project::compile_project_graph_inner(graph, entry)
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
            self.known_struct_layouts.clear();
            self.inlinable_methods.clear();
            self.inlinable_functions.clear();
        }
        self.lifted_functions.clear();
        self.fn_lit_counter = 0;
        self.method_name_ids.clear();
        self.method_names.clear();
        self.struct_shape_ids.clear();
        self.struct_shapes.clear();
        self.inlinable_functions.clear();
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
        for func in &program.functions {
            let q = self.qualify_local_fn_name(&func.name);
            if let Some(pattern) = Self::detect_inlinable_function(func) {
                self.inlinable_functions.insert(q, pattern);
            }
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
        for s in &program.structs {
            let runtime = self.resolve_struct_runtime_name(&s.name);
            let mut layout = StructLayout::default();
            for (slot, field) in s.fields.iter().enumerate() {
                layout.field_slots.insert(field.name.clone(), slot);
                if let TypeName::Named(type_name) = &field.ty {
                    layout.field_named_types.insert(
                        field.name.clone(),
                        self.resolve_struct_runtime_name(type_name),
                    );
                }
            }
            self.known_struct_layouts.insert(runtime, layout);
        }
        for imp in &program.impls {
            let target_name = self.resolve_struct_runtime_name(&imp.target);
            for method in &imp.methods {
                let mangled = Self::mangle_method_name(&target_name, &method.name);
                if let Some(pattern) =
                    Self::detect_inlinable_method(method, &target_name, &self.known_struct_layouts)
                {
                    self.inlinable_methods.insert(mangled, pattern);
                }
            }
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
        module.method_names = std::mem::take(&mut self.method_names);
        module.struct_shapes = std::mem::take(&mut self.struct_shapes);
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
            ctx.alloc_local_with_type(param.name.clone(), &param.ty);
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
            let ty = match &param.ty {
                TypeName::Named(type_name) => {
                    TypeName::Named(self.resolve_struct_runtime_name(type_name))
                }
                other => other.clone(),
            };
            ctx.alloc_local_with_type(param.name.clone(), &ty);
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
            let ty = match &param.ty {
                TypeName::Named(type_name) => {
                    TypeName::Named(self.resolve_struct_runtime_name(type_name))
                }
                other => other.clone(),
            };
            ctx.alloc_local_with_type(param.name.clone(), &ty);
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
            Expr::StringLit(v) => {
                code.push(Instr::LoadConst(Value::String(Rc::<str>::from(v.clone()))))
            }
            Expr::Ident(name) => {
                if let Some(slot) = ctx.lookup(name) {
                    code.push(Instr::LoadLocal(slot));
                } else if let Some(slot) = self.global_slots.get(name).copied() {
                    code.push(Instr::LoadGlobal(slot));
                } else if let Some(target) = self.direct_import_calls.get(name).cloned() {
                    code.push(Instr::LoadConst(Value::Function(Rc::<str>::from(target))));
                } else if let Some(target) = self.local_fn_qualified.get(name).cloned() {
                    code.push(Instr::LoadConst(Value::Function(Rc::<str>::from(target))));
                } else if self.function_names.contains(name) {
                    code.push(Instr::LoadConst(Value::Function(Rc::<str>::from(
                        name.clone(),
                    ))));
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
                    if let Some(instr) = Self::specialized_local_const_expr(op, left, right, ctx) {
                        code.push(instr);
                    } else if let Some(instr) =
                        Self::specialized_local_local_expr(op, left, right, ctx)
                    {
                        code.push(instr);
                    } else if let Some((left_expr, instr)) =
                        Self::specialized_stack_const_expr(op, left, right, ctx)
                    {
                        self.compile_expr(left_expr, ctx, code);
                        code.push(instr);
                    } else {
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
                }
            },
            Expr::Call { callee, args } => self.compile_call_expr(callee, args, ctx, code),
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
                if let Expr::Ident(name) = base.as_ref()
                    && let Some(slot) = ctx.lookup(name)
                {
                    self.compile_expr(index, ctx, code);
                    code.push(Instr::ArrayGetLocal(slot));
                } else {
                    self.compile_expr(base, ctx, code);
                    self.compile_expr(index, ctx, code);
                    code.push(Instr::ArrayGet);
                }
            }
            Expr::Field { .. } => {
                if let Some((base, fields)) = Self::flatten_field_expr(expr) {
                    if let Expr::Ident(name) = base
                        && let Some(local_slot) = ctx.lookup(name)
                        && let Some(base_ty) = Self::infer_expr_named_type(base, ctx)
                        && let Some(slots) = self.resolve_field_slots(&base_ty, &fields)
                        && let Some((first, rest)) = slots.split_first()
                    {
                        code.push(Instr::StructGetLocalSlot {
                            slot: local_slot,
                            field_slot: *first,
                        });
                        for slot in rest {
                            code.push(Instr::StructGetSlot(*slot));
                        }
                    } else {
                        self.compile_expr(base, ctx, code);
                        if let Some(base_ty) = Self::infer_expr_named_type(base, ctx)
                            && let Some(slots) = self.resolve_field_slots(&base_ty, &fields)
                        {
                            for slot in slots {
                                code.push(Instr::StructGetSlot(slot));
                            }
                        } else {
                            for field in fields {
                                code.push(Instr::StructGet(field));
                            }
                        }
                    }
                } else {
                    self.error("Unsupported field access shape in bytecode compiler".to_string());
                }
            }
            Expr::StructLit { name, fields } => {
                for (_, value) in fields {
                    self.compile_expr(value, ctx, code);
                }
                let runtime_name = self.resolve_struct_runtime_name(name);
                let field_names = fields.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>();
                if let Some(id) = self.intern_struct_shape(&runtime_name, &field_names) {
                    code.push(Instr::MakeStructId { id });
                } else {
                    code.push(Instr::MakeStruct {
                        name: runtime_name,
                        fields: field_names,
                    });
                }
            }
            Expr::FnLit { params, body, .. } => {
                let fn_name = self.compile_fn_lit(params, body);
                code.push(Instr::LoadConst(Value::Function(Rc::<str>::from(fn_name))));
            }
        }
    }

    fn error(&mut self, message: String) {
        self.diags.error(message, Span::default());
    }

    fn compile_cond_jump_false(
        &mut self,
        cond: &Expr,
        ctx: &mut FnCtx,
        code: &mut Vec<Instr>,
    ) -> usize {
        if let Some(instr) = Self::specialized_cond_jump_false(cond, ctx) {
            let at = code.len();
            code.push(instr);
            at
        } else {
            self.compile_expr(cond, ctx, code);
            let at = code.len();
            code.push(Instr::JumpIfFalse(usize::MAX));
            at
        }
    }

    fn patch_jump_false_target(code: &mut [Instr], at: usize, target: usize) {
        match &mut code[at] {
            Instr::JumpIfFalse(existing) => *existing = target,
            Instr::JumpIfLocalLtConst {
                target: existing, ..
            } => *existing = target,
            instr => unreachable!("expected jump-false instruction, found {instr:?}"),
        }
    }

    fn intern_method_name(&mut self, name: &str) -> usize {
        if let Some(id) = self.method_name_ids.get(name).copied() {
            return id;
        }
        let id = self.method_names.len();
        self.method_names.push(name.to_string());
        self.method_name_ids.insert(name.to_string(), id);
        id
    }

    fn intern_struct_shape(&mut self, runtime_name: &str, field_names: &[String]) -> Option<usize> {
        let known = self.known_struct_layouts.get(runtime_name)?;
        if field_names.len() != known.field_slots.len() {
            return None;
        }
        for (slot, field_name) in field_names.iter().enumerate() {
            if known.field_slots.get(field_name).copied() != Some(slot) {
                return None;
            }
        }
        if let Some(id) = self.struct_shape_ids.get(runtime_name).copied() {
            return Some(id);
        }
        let id = self.struct_shapes.len();
        self.struct_shapes.push(StructShape {
            name: runtime_name.to_string(),
            field_names: Rc::<[String]>::from(field_names.to_vec()),
        });
        self.struct_shape_ids.insert(runtime_name.to_string(), id);
        Some(id)
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

    fn detect_inlinable_function(func: &crate::ast::FnDecl) -> Option<InlinableFunction> {
        if func.params.len() != 1 {
            return None;
        }
        let param_name = &func.params[0].name;
        let [Stmt::Return(Some(expr))] = func.body.as_slice() else {
            return None;
        };
        let expr = Self::strip_groups(expr);
        let Expr::Binary { left, op, right } = expr else {
            return None;
        };
        if *op != BinaryOp::Add {
            return None;
        }
        match (Self::strip_groups(left), Self::strip_groups(right)) {
            (Expr::Ident(name), Expr::IntLit(rhs)) if name == param_name => {
                Some(InlinableFunction::AddConst(*rhs))
            }
            (Expr::IntLit(rhs), Expr::Ident(name)) if name == param_name => {
                Some(InlinableFunction::AddConst(*rhs))
            }
            _ => None,
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

    fn resolve_field_slots(&self, base_type: &str, fields: &[String]) -> Option<Vec<usize>> {
        let mut current = self.resolve_struct_runtime_name(base_type);
        let mut slots = Vec::with_capacity(fields.len());
        for field in fields {
            let layout = self.known_struct_layouts.get(&current)?;
            let slot = *layout.field_slots.get(field)?;
            slots.push(slot);
            let Some(next) = layout.field_named_types.get(field) else {
                if field != fields.last()? {
                    return None;
                }
                break;
            };
            current = next.clone();
        }
        Some(slots)
    }

    fn detect_inlinable_method(
        method: &crate::ast::MethodDecl,
        target_name: &str,
        layouts: &HashMap<String, StructLayout>,
    ) -> Option<InlinableMethod> {
        let layout = layouts.get(target_name)?;
        if method.params.len() != 2 {
            return None;
        }
        let arg_name = &method.params[1].name;
        let [Stmt::Return(Some(expr))] = method.body.as_slice() else {
            return None;
        };
        if let Some(field_name) = Self::match_self_field_plus_arg(expr, arg_name) {
            return Some(InlinableMethod::StructFieldAdd {
                field_slot: *layout.field_slots.get(field_name)?,
            });
        }
        let (lhs_field, rhs_field, mul, modulo) =
            Self::match_self_field_add_mul_field_mod(expr, arg_name)?;
        Some(InlinableMethod::StructFieldAddMulFieldMod {
            lhs_field_slot: *layout.field_slots.get(lhs_field)?,
            rhs_field_slot: *layout.field_slots.get(rhs_field)?,
            mul,
            modulo,
        })
    }

    fn match_self_field_plus_arg<'a>(expr: &'a Expr, arg_name: &str) -> Option<&'a str> {
        let expr = Self::strip_groups(expr);
        let Expr::Binary { left, op, right } = expr else {
            return None;
        };
        if *op != BinaryOp::Add {
            return None;
        }
        match (Self::strip_groups(left), Self::strip_groups(right)) {
            (Expr::Field { base, field }, Expr::Ident(arg)) if arg == arg_name => {
                let Expr::Ident(self_name) = Self::strip_groups(base) else {
                    return None;
                };
                if self_name == "self" {
                    Some(field.as_str())
                } else {
                    None
                }
            }
            (Expr::Ident(arg), Expr::Field { base, field }) if arg == arg_name => {
                let Expr::Ident(self_name) = Self::strip_groups(base) else {
                    return None;
                };
                if self_name == "self" {
                    Some(field.as_str())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn match_self_field_add_mul_field_mod<'a>(
        expr: &'a Expr,
        arg_name: &str,
    ) -> Option<(&'a str, &'a str, i64, i64)> {
        let expr = Self::strip_groups(expr);
        let Expr::Binary {
            left: mod_left,
            op: mod_op,
            right: mod_right,
        } = expr
        else {
            return None;
        };
        if *mod_op != BinaryOp::Mod {
            return None;
        }
        let Expr::IntLit(modulo) = Self::strip_groups(mod_right) else {
            return None;
        };
        let mod_left = Self::strip_groups(mod_left);
        let Expr::Binary {
            left: add_left,
            op: add_op,
            right: add_right,
        } = mod_left
        else {
            return None;
        };
        if *add_op != BinaryOp::Add {
            return None;
        }
        let Expr::Field {
            base: rhs_base,
            field: rhs_field,
        } = Self::strip_groups(add_right)
        else {
            return None;
        };
        let Expr::Ident(rhs_self) = Self::strip_groups(rhs_base) else {
            return None;
        };
        if rhs_self != "self" {
            return None;
        }
        let add_left = Self::strip_groups(add_left);
        let Expr::Binary {
            left: mul_left,
            op: mul_op,
            right: mul_right,
        } = add_left
        else {
            return None;
        };
        if *mul_op != BinaryOp::Mul {
            return None;
        }
        let Expr::IntLit(mul) = Self::strip_groups(mul_right) else {
            return None;
        };
        let lhs_field = Self::match_self_field_plus_arg(mul_left, arg_name)?;
        Some((lhs_field, rhs_field.as_str(), *mul, *modulo))
    }

    fn strip_groups(mut expr: &Expr) -> &Expr {
        while let Expr::Group(inner) = expr {
            expr = inner;
        }
        expr
    }
}
