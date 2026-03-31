use std::collections::HashMap;

use crate::ast::{FnDecl, MethodDecl, OperatorDecl, Program, StructDecl};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::ir::{Instr, IrProgram, IrType, IrVerifier, Terminator, opt};
use crate::parser::Parser;
use crate::resolver::{ModuleGraph, SymbolKind};

mod context;
mod expr;
mod expr_calls;
mod expr_helpers;
mod project;
mod stmt;

use context::{ExternFunctionSig, FunctionLowering, FunctionSig, IrLowerer};

pub use project::{
    compile_project_entry, compile_project_entry_unoptimized, compile_project_graph,
    compile_project_graph_unoptimized,
};

pub fn compile_source(source: &str) -> Result<IrProgram, DiagnosticBag> {
    let mut ir = compile_source_unoptimized(source)?;
    opt::optimize_program(&mut ir);
    Ok(ir)
}

pub fn compile_source_unoptimized(source: &str) -> Result<IrProgram, DiagnosticBag> {
    let (program, mut diags) = Parser::parse_source(source);
    if !diags.is_empty() {
        return Err(diags);
    }

    let mut lowerer = IrLowerer::new();
    let ir = lowerer.compile_program(&program);
    for diag in lowerer.diags.into_vec() {
        diags.push(diag);
    }

    if diags.is_empty() {
        match IrVerifier::verify_program(&ir) {
            Ok(()) => Ok(ir),
            Err(err) => {
                diags.error(format!("IR verification failed: {err:?}"), Span::default());
                Err(diags)
            }
        }
    } else {
        Err(diags)
    }
}

impl IrLowerer {
    fn configure_project_module(
        &mut self,
        module_id: &str,
        program: &Program,
        graph: &ModuleGraph,
        export_maps: &HashMap<String, HashMap<String, crate::resolver::SymbolRef>>,
    ) {
        self.module_id = Some(module_id.to_string());
        self.direct_import_calls.clear();
        self.imported_global_names.clear();
        self.imported_struct_runtime.clear();
        self.namespace_call_targets.clear();

        for strukt in &program.structs {
            self.imported_struct_runtime
                .insert(strukt.name.clone(), format!("{module_id}::{}", strukt.name));
        }

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
                                SymbolKind::Fn => {
                                    self.direct_import_calls.insert(
                                        name.clone(),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                                SymbolKind::Struct => {
                                    self.imported_struct_runtime.insert(
                                        name.clone(),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                                SymbolKind::GlobalLet => {
                                    self.imported_global_names.insert(
                                        name.clone(),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                                SymbolKind::Namespace => {}
                            }
                        }
                    } else {
                        for item in items {
                            let local = item.alias.clone().unwrap_or_else(|| item.name.clone());
                            let Some(sym) = exports.get(&item.name) else {
                                continue;
                            };
                            match sym.kind {
                                SymbolKind::Fn => {
                                    self.direct_import_calls.insert(
                                        local,
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                                SymbolKind::Struct => {
                                    self.imported_struct_runtime.insert(
                                        local,
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                                SymbolKind::GlobalLet => {
                                    self.imported_global_names.insert(
                                        local,
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                                SymbolKind::Namespace => {}
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
                                if sym.kind == SymbolKind::Fn {
                                    self.namespace_call_targets.insert(
                                        format!("{mid}.{ename}"),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                    self.namespace_call_targets.insert(
                                        format!("{ns}.{ename}"),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                                if sym.kind == SymbolKind::Struct {
                                    self.imported_struct_runtime.insert(
                                        format!("{mid}.{ename}"),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                    self.imported_struct_runtime.insert(
                                        format!("{ns}.{ename}"),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                                if sym.kind == SymbolKind::GlobalLet {
                                    self.imported_global_names.insert(
                                        format!("{mid}.{ename}"),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                    self.imported_global_names.insert(
                                        format!("{ns}.{ename}"),
                                        format!("{}::{}", sym.module_id, sym.local_name),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        let _ = graph;
    }

    fn qualify_name(&self, name: &str) -> String {
        match &self.module_id {
            Some(module_id) => format!("{module_id}::{name}"),
            None => name.to_string(),
        }
    }

    fn resolve_struct_runtime_name(&self, name: &str) -> String {
        self.imported_struct_runtime
            .get(name)
            .cloned()
            .unwrap_or_else(|| self.qualify_name(name))
    }

    fn lower_type_name(&self, ty: &crate::ast::TypeName) -> IrType {
        match ty {
            crate::ast::TypeName::Int => IrType::Int,
            crate::ast::TypeName::Float => IrType::Float,
            crate::ast::TypeName::Bool => IrType::Bool,
            crate::ast::TypeName::String => IrType::String,
            crate::ast::TypeName::Bytes => IrType::Bytes,
            crate::ast::TypeName::Void => IrType::Void,
            crate::ast::TypeName::Named(name) => {
                if crate::types::is_builtin_opaque_type(name) {
                    IrType::Opaque(name.clone())
                } else {
                    IrType::Named(self.resolve_struct_runtime_name(name))
                }
            }
            crate::ast::TypeName::Array { elem, size } => IrType::Array {
                elem: Box::new(self.lower_type_name(elem)),
                size: *size,
            },
            crate::ast::TypeName::Vec { elem } => IrType::Vec {
                elem: Box::new(self.lower_type_name(elem)),
            },
            crate::ast::TypeName::Map { value } => IrType::Map {
                value: Box::new(self.lower_type_name(value)),
            },
            crate::ast::TypeName::Fn { params, ret } => IrType::Fn {
                params: params.iter().map(|p| self.lower_type_name(p)).collect(),
                ret: Box::new(self.lower_type_name(ret)),
            },
        }
    }

    fn compile_program(&mut self, program: &Program) -> IrProgram {
        let mut out = self.builder.begin_program();
        self.compile_program_into(program, &mut out);
        out.functions.append(&mut self.lifted_functions);
        out
    }

    fn compile_program_into(&mut self, program: &Program, out: &mut IrProgram) {
        self.register_program_items(program, out);
        self.lower_program_bodies(program, out);
    }

    fn register_program_items(&mut self, program: &Program, out: &mut IrProgram) {
        for strukt in &program.structs {
            let id = crate::ir::StructId(self.structs.len());
            let fields = self.lower_struct_fields(strukt);
            let runtime_name = self.resolve_struct_runtime_name(&strukt.name);
            self.structs
                .insert(runtime_name.clone(), (id, fields.clone()));
            out.structs.push(crate::ir::IrStruct {
                id,
                name: runtime_name,
                fields,
            });
        }

        for global in &program.globals {
            let ty = global
                .ty
                .as_ref()
                .map(|ty| self.lower_type_name(ty))
                .unwrap_or(IrType::Unknown);
            let id = crate::ir::GlobalId(self.globals.len());
            self.globals
                .insert(self.qualify_name(&global.name), (id, ty.clone()));
            out.globals.push(crate::ir::IrGlobal {
                id,
                name: self.qualify_name(&global.name),
                ty,
                init: None,
            });
        }

        if !program.globals.is_empty() {
            let init_name = self.qualify_name("__globals_init");
            let id = crate::ir::FunctionId(self.functions.len());
            self.functions.insert(
                init_name,
                FunctionSig {
                    id,
                    params: Vec::new(),
                    ret: IrType::Void,
                },
            );
        }

        for func in &program.functions {
            let params = func
                .params
                .iter()
                .map(|param| self.lower_type_name(&param.ty))
                .collect::<Vec<_>>();
            let ret_ty = func
                .return_type
                .as_ref()
                .map(|ty| self.lower_type_name(ty))
                .unwrap_or(IrType::Void);
            if func.is_extern {
                self.extern_functions.insert(
                    self.qualify_name(&func.name),
                    ExternFunctionSig {
                        library: func.extern_library.clone(),
                        symbol: func.name.clone(),
                        params: func
                            .params
                            .iter()
                            .map(|param| self.lower_type_name(&param.ty))
                            .collect(),
                        ret: func
                            .return_type
                            .as_ref()
                            .map(|ty| self.lower_type_name(ty))
                            .unwrap_or(IrType::Void),
                    },
                );
                continue;
            }
            let id = crate::ir::FunctionId(self.functions.len());
            self.functions.insert(
                self.qualify_name(&func.name),
                FunctionSig {
                    id,
                    params,
                    ret: ret_ty,
                },
            );
        }

        for operator in &program.operators {
            let params = operator
                .params
                .iter()
                .map(|param| self.lower_type_name(&param.ty))
                .collect::<Vec<_>>();
            let ret_ty = self.lower_type_name(&operator.return_type);
            let id = crate::ir::FunctionId(self.functions.len());
            self.functions.insert(
                self.qualify_name(&operator.name),
                FunctionSig {
                    id,
                    params,
                    ret: ret_ty,
                },
            );
        }

        for imp in &program.impls {
            for method in &imp.methods {
                let params = method
                    .params
                    .iter()
                    .map(|param| {
                        if param.name == "self" {
                            IrType::Named(self.resolve_struct_runtime_name(&imp.target))
                        } else {
                            self.lower_type_name(&param.ty)
                        }
                    })
                    .collect::<Vec<_>>();
                let ret_ty = method
                    .return_type
                    .as_ref()
                    .map(|ty| self.lower_type_name(ty))
                    .unwrap_or(IrType::Void);
                let method_name = Self::mangle_method_name(
                    &self.resolve_struct_runtime_name(&imp.target),
                    &method.name,
                );
                let id = crate::ir::FunctionId(self.functions.len());
                self.functions.insert(
                    method_name,
                    FunctionSig {
                        id,
                        params,
                        ret: ret_ty,
                    },
                );
            }
        }
    }

    fn lower_program_bodies(&mut self, program: &Program, out: &mut IrProgram) {
        if !program.globals.is_empty() {
            let mut init = self
                .builder
                .begin_function(self.qualify_name("__globals_init"), IrType::Void);
            let init_id = self
                .functions
                .get(&init.name)
                .map(|sig| sig.id)
                .unwrap_or(crate::ir::FunctionId(usize::MAX));
            init.id = init_id;
            if self.compile_globals_init(&mut init, program).is_some() {
                if !self.project_mode {
                    out.module_init = Some(crate::ir::IrModuleInit { function: init.id });
                }
                out.functions.push(init);
            }
        }

        for func in &program.functions {
            if func.is_extern {
                continue;
            }
            if let Some(lowered) = self.compile_function(func) {
                out.functions.push(lowered);
            }
        }

        for operator in &program.operators {
            if let Some(lowered) = self.compile_operator(operator) {
                out.functions.push(lowered);
            }
        }

        for imp in &program.impls {
            for method in &imp.methods {
                if let Some(lowered) = self.compile_method(&imp.target, method) {
                    out.functions.push(lowered);
                }
            }
        }
    }

    fn lower_struct_fields(&self, strukt: &StructDecl) -> Vec<crate::ir::StructField> {
        strukt
            .fields
            .iter()
            .map(|field| crate::ir::StructField {
                name: field.name.clone(),
                ty: self.lower_type_name(&field.ty),
            })
            .collect()
    }

    fn compile_function(&mut self, func: &FnDecl) -> Option<crate::ir::IrFunction> {
        let sig = self
            .functions
            .get(&self.qualify_name(&func.name))
            .cloned()
            .unwrap_or(FunctionSig {
                id: crate::ir::FunctionId(usize::MAX),
                params: Vec::new(),
                ret: IrType::Void,
            });
        let mut out = self
            .builder
            .begin_function(self.qualify_name(&func.name), sig.ret.clone());
        out.id = sig.id;
        let mut lowering = FunctionLowering {
            current_block: out.entry,
            locals: HashMap::new(),
            scratch_counter: 0,
            loops: Vec::new(),
        };

        for param in &func.params {
            self.builder.push_param(
                &mut out,
                param.name.clone(),
                self.lower_type_name(&param.ty),
            );
            let local = self.builder.push_local(
                &mut out,
                param.name.clone(),
                self.lower_type_name(&param.ty),
            );
            lowering.locals.insert(param.name.clone(), local);
        }

        for stmt in &func.body {
            if !self.compile_stmt(&mut out, &mut lowering, stmt) {
                return None;
            }
        }

        if matches!(
            out.blocks
                .iter()
                .find(|block| block.id == lowering.current_block)
                .map(|block| &block.terminator),
            Some(Terminator::Unreachable)
        ) {
            let terminator = if sig.ret.is_void() {
                Terminator::Return(None)
            } else {
                self.diags.error(
                    format!(
                        "IR lowering currently requires explicit return in non-void function `{}`",
                        func.name
                    ),
                    Span::default(),
                );
                return None;
            };
            self.builder
                .set_terminator(&mut out, lowering.current_block, terminator);
        }

        Some(out)
    }

    fn compile_operator(&mut self, operator: &OperatorDecl) -> Option<crate::ir::IrFunction> {
        let sig = self
            .functions
            .get(&self.qualify_name(&operator.name))
            .cloned()
            .unwrap_or(FunctionSig {
                id: crate::ir::FunctionId(usize::MAX),
                params: Vec::new(),
                ret: IrType::Void,
            });
        let mut out = self
            .builder
            .begin_function(self.qualify_name(&operator.name), sig.ret.clone());
        out.id = sig.id;
        let mut lowering = FunctionLowering {
            current_block: out.entry,
            locals: HashMap::new(),
            scratch_counter: 0,
            loops: Vec::new(),
        };

        for param in &operator.params {
            self.builder.push_param(
                &mut out,
                param.name.clone(),
                self.lower_type_name(&param.ty),
            );
            let local = self.builder.push_local(
                &mut out,
                param.name.clone(),
                self.lower_type_name(&param.ty),
            );
            lowering.locals.insert(param.name.clone(), local);
        }

        for stmt in &operator.body {
            if !self.compile_stmt(&mut out, &mut lowering, stmt) {
                return None;
            }
        }

        if matches!(
            out.blocks
                .iter()
                .find(|block| block.id == lowering.current_block)
                .map(|block| &block.terminator),
            Some(Terminator::Unreachable)
        ) {
            self.diags.error(
                format!(
                    "IR lowering currently requires explicit return in operator `{}`",
                    operator.name
                ),
                Span::default(),
            );
            return None;
        }

        Some(out)
    }

    fn compile_method(
        &mut self,
        target: &str,
        method: &MethodDecl,
    ) -> Option<crate::ir::IrFunction> {
        let runtime_target = self.resolve_struct_runtime_name(target);
        let method_name = Self::mangle_method_name(&runtime_target, &method.name);
        let sig = self
            .functions
            .get(&method_name)
            .cloned()
            .unwrap_or(FunctionSig {
                id: crate::ir::FunctionId(usize::MAX),
                params: Vec::new(),
                ret: IrType::Void,
            });
        let mut out = self.builder.begin_function(method_name, sig.ret.clone());
        out.id = sig.id;
        let mut lowering = FunctionLowering {
            current_block: out.entry,
            locals: HashMap::new(),
            scratch_counter: 0,
            loops: Vec::new(),
        };

        for param in &method.params {
            let ir_ty = if param.name == "self" {
                IrType::Named(runtime_target.clone())
            } else {
                self.lower_type_name(&param.ty)
            };
            self.builder
                .push_param(&mut out, param.name.clone(), ir_ty.clone());
            let local = self.builder.push_local(&mut out, param.name.clone(), ir_ty);
            lowering.locals.insert(param.name.clone(), local);
        }

        for stmt in &method.body {
            if !self.compile_stmt(&mut out, &mut lowering, stmt) {
                return None;
            }
        }

        if matches!(
            out.blocks
                .iter()
                .find(|block| block.id == lowering.current_block)
                .map(|block| &block.terminator),
            Some(Terminator::Unreachable)
        ) {
            let terminator = if sig.ret.is_void() {
                Terminator::Return(None)
            } else {
                self.diags.error(
                    format!(
                        "IR lowering currently requires explicit return in non-void method `{}`",
                        method.name
                    ),
                    Span::default(),
                );
                return None;
            };
            self.builder
                .set_terminator(&mut out, lowering.current_block, terminator);
        }

        Some(out)
    }

    fn compile_globals_init(
        &mut self,
        func: &mut crate::ir::IrFunction,
        program: &Program,
    ) -> Option<()> {
        let mut lowering = FunctionLowering {
            current_block: func.entry,
            locals: HashMap::new(),
            scratch_counter: 0,
            loops: Vec::new(),
        };

        for global in &program.globals {
            let value = self.compile_expr(func, &mut lowering, &global.value)?;
            let Some((id, ty)) = self.globals.get(&self.qualify_name(&global.name)).cloned() else {
                self.unsupported(format!("global `{}` was not registered", global.name));
                return None;
            };
            self.builder.push_instr(
                func,
                lowering.current_block,
                Instr::StoreGlobal {
                    global: id,
                    ty,
                    value,
                },
            );
        }

        self.builder
            .set_terminator(func, lowering.current_block, Terminator::Return(None));
        Some(())
    }

    fn unsupported(&mut self, message: impl Into<String>) {
        self.diags.error(message, Span::default());
    }

    fn mangle_method_name(target: &str, method: &str) -> String {
        format!("{target}::{method}")
    }
}
