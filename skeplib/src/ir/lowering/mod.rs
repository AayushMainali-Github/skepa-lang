use std::collections::HashMap;
use std::path::Path;

use crate::ast::{
    AssignTarget, BinaryOp as AstBinaryOp, Expr, FnDecl, MethodDecl, Program, Stmt, StructDecl,
    UnaryOp as AstUnaryOp,
};
use crate::diagnostic::{DiagnosticBag, Span};
use crate::ir::{
    BinaryOp, BlockId, BranchTerminator, ConstValue, Instr, IrBuilder, IrProgram, IrType,
    IrVerifier, Operand, Terminator, UnaryOp,
};
use crate::parser::Parser;
use crate::resolver::{
    ModuleGraph, ResolveError, ResolveErrorKind, SymbolKind, build_export_maps, resolve_project,
};
use crate::types::TypeInfo;

pub fn compile_source(source: &str) -> Result<IrProgram, DiagnosticBag> {
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

pub fn compile_project_entry(entry: &Path) -> Result<IrProgram, Vec<ResolveError>> {
    let graph = resolve_project(entry)?;
    compile_project_graph(&graph, entry).map_err(|e| {
        vec![ResolveError::new(
            ResolveErrorKind::Codegen,
            e,
            Some(entry.to_path_buf()),
        )]
    })
}

pub fn compile_project_graph(graph: &ModuleGraph, entry: &Path) -> Result<IrProgram, String> {
    let export_maps = build_export_maps(graph).map_err(|errs| errs[0].message.clone())?;
    let entry_path = entry.canonicalize().unwrap_or_else(|_| entry.to_path_buf());
    let Some((entry_id, _)) = graph.modules.iter().find(|(_, m)| {
        m.path == entry
            || m.path == entry_path
            || m.path
                .canonicalize()
                .map(|p| p == entry_path)
                .unwrap_or(false)
    }) else {
        return Err("Entry module missing from graph".to_string());
    };

    let mut lowerer = IrLowerer::new_project();
    let mut out = lowerer.builder.begin_program();
    let mut init_function_ids = Vec::new();
    let mut modules = Vec::new();
    let mut ids = graph.modules.keys().cloned().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        let module = &graph.modules[&id];
        let (program, diags) = Parser::parse_source(&module.source);
        if !diags.is_empty() {
            return Err(format!(
                "Parse failed for {}: {:?}",
                module.path.display(),
                diags
            ));
        }
        modules.push((id, program));
    }

    for (id, program) in &modules {
        lowerer.configure_project_module(id, program, graph, &export_maps);
        lowerer.register_program_items(program, &mut out);
    }

    for (id, program) in &modules {
        lowerer.configure_project_module(id, program, graph, &export_maps);
        let init_name = lowerer.qualify_name("__globals_init");
        lowerer.lower_program_bodies(program, &mut out);
        if let Some((function_id, _)) = lowerer.functions.get(&init_name).cloned()
            && out.functions.iter().any(|func| func.id == function_id)
            && !init_function_ids.contains(&function_id)
        {
            init_function_ids.push(function_id);
        }
    }

    if !init_function_ids.is_empty() {
        let wrapper_id = crate::ir::FunctionId(lowerer.functions.len());
        lowerer
            .functions
            .insert("__globals_init".to_string(), (wrapper_id, IrType::Void));
        let mut init = lowerer
            .builder
            .begin_function("__globals_init", IrType::Void);
        init.id = wrapper_id;
        let init_entry = init.entry;
        for function in init_function_ids {
            lowerer.builder.push_instr(
                &mut init,
                init_entry,
                Instr::CallDirect {
                    dst: None,
                    ret_ty: IrType::Void,
                    function,
                    args: Vec::new(),
                },
            );
        }
        lowerer
            .builder
            .set_terminator(&mut init, init_entry, Terminator::Return(None));
        out.module_init = Some(crate::ir::IrModuleInit { function: init.id });
        out.functions.push(init);
    }

    let entry_main_name = format!("{entry_id}::main");
    let Some((entry_main_id, entry_main_ty)) = out
        .functions
        .iter()
        .find(|func| func.name == entry_main_name)
        .map(|func| (func.id, func.ret_ty.clone()))
    else {
        return Err("Entry module does not define main".to_string());
    };
    let wrapper_main_id = crate::ir::FunctionId(lowerer.functions.len());
    lowerer
        .functions
        .insert("main".to_string(), (wrapper_main_id, entry_main_ty.clone()));
    let mut main = lowerer
        .builder
        .begin_function("main", entry_main_ty.clone());
    main.id = wrapper_main_id;
    let main_entry = main.entry;
    let dst = if entry_main_ty.is_void() {
        None
    } else {
        Some(lowerer.builder.push_temp(&mut main, entry_main_ty.clone()))
    };
    lowerer.builder.push_instr(
        &mut main,
        main_entry,
        Instr::CallDirect {
            dst,
            ret_ty: entry_main_ty,
            function: entry_main_id,
            args: Vec::new(),
        },
    );
    lowerer.builder.set_terminator(
        &mut main,
        main_entry,
        Terminator::Return(dst.map(Operand::Temp)),
    );
    out.functions.push(main);
    out.functions.append(&mut lowerer.lifted_functions);

    IrVerifier::verify_program(&out).map_err(|err| format!("IR verification failed: {err:?}"))?;
    Ok(out)
}

struct IrLowerer {
    builder: IrBuilder,
    diags: DiagnosticBag,
    functions: HashMap<String, (crate::ir::FunctionId, IrType)>,
    globals: HashMap<String, (crate::ir::GlobalId, IrType)>,
    structs: HashMap<String, (crate::ir::StructId, Vec<crate::ir::StructField>)>,
    module_id: Option<String>,
    direct_import_calls: HashMap<String, String>,
    imported_struct_runtime: HashMap<String, String>,
    namespace_call_targets: HashMap<String, String>,
    project_mode: bool,
    lifted_functions: Vec<crate::ir::IrFunction>,
    fn_lit_counter: usize,
}

struct FunctionLowering {
    current_block: BlockId,
    locals: HashMap<String, crate::ir::LocalId>,
    scratch_counter: usize,
}

impl IrLowerer {
    fn new() -> Self {
        Self {
            builder: IrBuilder::new(),
            diags: DiagnosticBag::new(),
            functions: HashMap::new(),
            globals: HashMap::new(),
            structs: HashMap::new(),
            module_id: None,
            direct_import_calls: HashMap::new(),
            imported_struct_runtime: HashMap::new(),
            namespace_call_targets: HashMap::new(),
            project_mode: false,
            lifted_functions: Vec::new(),
            fn_lit_counter: 0,
        }
    }

    fn new_project() -> Self {
        let mut this = Self::new();
        this.project_mode = true;
        this
    }

    fn configure_project_module(
        &mut self,
        module_id: &str,
        program: &Program,
        graph: &ModuleGraph,
        export_maps: &HashMap<String, HashMap<String, crate::resolver::SymbolRef>>,
    ) {
        self.module_id = Some(module_id.to_string());
        self.direct_import_calls.clear();
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
                                    self.globals.entry(name.clone()).or_insert((
                                        crate::ir::GlobalId(usize::MAX),
                                        IrType::Unknown,
                                    ));
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
                                SymbolKind::GlobalLet | SymbolKind::Namespace => {}
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
            crate::ast::TypeName::Void => IrType::Void,
            crate::ast::TypeName::Named(name) => {
                IrType::Named(self.resolve_struct_runtime_name(name))
            }
            crate::ast::TypeName::Array { elem, size } => IrType::Array {
                elem: Box::new(self.lower_type_name(elem)),
                size: *size,
            },
            crate::ast::TypeName::Vec { elem } => IrType::Vec {
                elem: Box::new(self.lower_type_name(elem)),
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

        for func in &program.functions {
            let ret_ty = func
                .return_type
                .as_ref()
                .map(|ty| self.lower_type_name(ty))
                .unwrap_or(IrType::Void);
            let id = crate::ir::FunctionId(self.functions.len());
            self.functions
                .insert(self.qualify_name(&func.name), (id, ret_ty));
        }

        for imp in &program.impls {
            for method in &imp.methods {
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
                self.functions.insert(method_name, (id, ret_ty));
            }
        }
    }

    fn lower_program_bodies(&mut self, program: &Program, out: &mut IrProgram) {
        if !program.globals.is_empty() {
            let mut init = self
                .builder
                .begin_function(self.qualify_name("__globals_init"), IrType::Void);
            let (init_id, _) = self
                .functions
                .get(&init.name)
                .cloned()
                .unwrap_or((crate::ir::FunctionId(usize::MAX), IrType::Void));
            init.id = init_id;
            if self.compile_globals_init(&mut init, program).is_some() {
                if !self.project_mode {
                    out.module_init = Some(crate::ir::IrModuleInit { function: init.id });
                }
                out.functions.push(init);
            }
        }

        for func in &program.functions {
            if let Some(lowered) = self.compile_function(func) {
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
        let (function_id, ret_ty) = self
            .functions
            .get(&self.qualify_name(&func.name))
            .cloned()
            .unwrap_or((crate::ir::FunctionId(usize::MAX), IrType::Void));
        let mut out = self
            .builder
            .begin_function(self.qualify_name(&func.name), ret_ty.clone());
        out.id = function_id;
        let mut lowering = FunctionLowering {
            current_block: out.entry,
            locals: HashMap::new(),
            scratch_counter: 0,
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
            let terminator = if ret_ty.is_void() {
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

    fn compile_method(
        &mut self,
        target: &str,
        method: &MethodDecl,
    ) -> Option<crate::ir::IrFunction> {
        let runtime_target = self.resolve_struct_runtime_name(target);
        let method_name = Self::mangle_method_name(&runtime_target, &method.name);
        let (function_id, ret_ty) = self
            .functions
            .get(&method_name)
            .cloned()
            .unwrap_or((crate::ir::FunctionId(usize::MAX), IrType::Void));
        let mut out = self.builder.begin_function(method_name, ret_ty.clone());
        out.id = function_id;
        let mut lowering = FunctionLowering {
            current_block: out.entry,
            locals: HashMap::new(),
            scratch_counter: 0,
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
            let terminator = if ret_ty.is_void() {
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

    fn compile_stmt(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        stmt: &Stmt,
    ) -> bool {
        match stmt {
            Stmt::Let { name, ty, value } => {
                if let Some(done) = self.try_compile_vec_new_let(func, lowering, name, ty, value) {
                    return done;
                }
                let rhs = match self.compile_expr(func, lowering, value) {
                    Some(value) => value,
                    None => return false,
                };
                let ir_ty = ty
                    .as_ref()
                    .map(|ty| self.lower_type_name(ty))
                    .unwrap_or_else(|| self.infer_operand_type(func, &rhs));
                let local = self.builder.push_local(func, name.clone(), ir_ty.clone());
                lowering.locals.insert(name.clone(), local);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::StoreLocal {
                        local,
                        ty: ir_ty,
                        value: rhs,
                    },
                );
                true
            }
            Stmt::Assign {
                target: AssignTarget::Ident(name),
                value,
            } => {
                let rhs = match self.compile_expr(func, lowering, value) {
                    Some(value) => value,
                    None => return false,
                };
                let Some(&local) = lowering.locals.get(name) else {
                    self.unsupported(format!("assignment to unknown local `{name}`"));
                    return false;
                };
                let ty = func
                    .locals
                    .iter()
                    .find(|entry| entry.id == local)
                    .map(|entry| entry.ty.clone())
                    .unwrap_or(IrType::Unknown);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::StoreLocal {
                        local,
                        ty,
                        value: rhs,
                    },
                );
                true
            }
            Stmt::Assign {
                target: AssignTarget::Index { base, index },
                value,
            } => {
                let array = match self.compile_expr(func, lowering, base) {
                    Some(value) => value,
                    None => return false,
                };
                let index = match self.compile_expr(func, lowering, index) {
                    Some(value) => value,
                    None => return false,
                };
                let value = match self.compile_expr(func, lowering, value) {
                    Some(value) => value,
                    None => return false,
                };
                let elem_ty = self.array_element_type(func, &array);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::ArraySet {
                        elem_ty,
                        array,
                        index,
                        value,
                    },
                );
                true
            }
            Stmt::Assign {
                target: AssignTarget::Field { base, field },
                value,
            } => {
                let base = match self.compile_expr(func, lowering, base) {
                    Some(value) => value,
                    None => return false,
                };
                let value = match self.compile_expr(func, lowering, value) {
                    Some(value) => value,
                    None => return false,
                };
                let ty = self.field_type(func, &base, field);
                let field_ref = self.resolve_field_ref(func, &base, field);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::StructSet {
                        base,
                        field: field_ref,
                        value,
                        ty,
                    },
                );
                true
            }
            Stmt::Expr(expr) => self.compile_expr(func, lowering, expr).is_some(),
            Stmt::Return(value) => {
                let ret = match value {
                    Some(expr) => match self.compile_expr(func, lowering, expr) {
                        Some(value) => Some(value),
                        None => return false,
                    },
                    None => None,
                };
                self.builder
                    .set_terminator(func, lowering.current_block, Terminator::Return(ret));
                true
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => self.compile_if(func, lowering, cond, then_body, else_body),
            Stmt::While { cond, body } => self.compile_while(func, lowering, cond, body),
            _ => {
                self.unsupported("statement form is not in the initial IR lowering subset");
                false
            }
        }
    }

    fn compile_if(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        cond: &Expr,
        then_body: &[Stmt],
        else_body: &[Stmt],
    ) -> bool {
        let cond_value = match self.compile_expr(func, lowering, cond) {
            Some(value) => value,
            None => return false,
        };
        let then_block = self.builder.push_block(func, "if_then");
        let else_block = self.builder.push_block(func, "if_else");
        let join_block = self.builder.push_block(func, "if_join");
        self.builder.set_terminator(
            func,
            lowering.current_block,
            Terminator::Branch(BranchTerminator {
                cond: cond_value,
                then_block,
                else_block,
            }),
        );

        lowering.current_block = then_block;
        for stmt in then_body {
            if !self.compile_stmt(func, lowering, stmt) {
                return false;
            }
        }
        self.ensure_fallthrough_jump(func, lowering.current_block, join_block);

        lowering.current_block = else_block;
        for stmt in else_body {
            if !self.compile_stmt(func, lowering, stmt) {
                return false;
            }
        }
        self.ensure_fallthrough_jump(func, lowering.current_block, join_block);

        lowering.current_block = join_block;
        true
    }

    fn compile_while(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        cond: &Expr,
        body: &[Stmt],
    ) -> bool {
        let cond_block = self.builder.push_block(func, "while_cond");
        let body_block = self.builder.push_block(func, "while_body");
        let exit_block = self.builder.push_block(func, "while_exit");

        self.builder
            .set_terminator(func, lowering.current_block, Terminator::Jump(cond_block));

        lowering.current_block = cond_block;
        let cond_value = match self.compile_expr(func, lowering, cond) {
            Some(value) => value,
            None => return false,
        };
        self.builder.set_terminator(
            func,
            cond_block,
            Terminator::Branch(BranchTerminator {
                cond: cond_value,
                then_block: body_block,
                else_block: exit_block,
            }),
        );

        lowering.current_block = body_block;
        for stmt in body {
            if !self.compile_stmt(func, lowering, stmt) {
                return false;
            }
        }
        self.ensure_fallthrough_jump(func, lowering.current_block, cond_block);

        lowering.current_block = exit_block;
        true
    }

    fn compile_expr(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        expr: &Expr,
    ) -> Option<Operand> {
        match expr {
            Expr::IntLit(value) => Some(Operand::Const(ConstValue::Int(*value))),
            Expr::BoolLit(value) => Some(Operand::Const(ConstValue::Bool(*value))),
            Expr::StringLit(value) => Some(Operand::Const(ConstValue::String(value.clone()))),
            Expr::Ident(name) => lowering
                .locals
                .get(name)
                .copied()
                .map(Operand::Local)
                .or_else(|| {
                    self.globals
                        .get(name)
                        .or_else(|| self.globals.get(&self.qualify_name(name)))
                        .map(|(id, _)| Operand::Global(*id))
                })
                .or_else(|| self.function_value(func, lowering.current_block, name))
                .or_else(|| {
                    self.unsupported(format!("reference to unresolved identifier `{name}`"));
                    None
                }),
            Expr::Path(parts) => {
                let name = parts.join(".");
                self.globals
                    .get(&name)
                    .or_else(|| self.globals.get(&self.qualify_name(&name)))
                    .map(|(id, _)| Operand::Global(*id))
                    .or_else(|| {
                        self.unsupported(format!(
                            "path `{name}` is not in the initial IR lowering subset"
                        ));
                        None
                    })
            }
            Expr::Field { base, field } => {
                let base = self.compile_expr(func, lowering, base)?;
                let ty = self.field_type(func, &base, field);
                let field_ref = self.resolve_field_ref(func, &base, field);
                let dst = self.builder.push_temp(func, ty.clone());
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::StructGet {
                        dst,
                        ty,
                        base,
                        field: field_ref,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::ArrayLit(items) => {
                let mut lowered_items = Vec::with_capacity(items.len());
                for item in items {
                    lowered_items.push(self.compile_expr(func, lowering, item)?);
                }
                let elem_ty = lowered_items
                    .first()
                    .map(|item| self.infer_operand_type(func, item))
                    .unwrap_or(IrType::Unknown);
                let ty = IrType::Array {
                    elem: Box::new(elem_ty.clone()),
                    size: lowered_items.len(),
                };
                let dst = self.builder.push_temp(func, ty);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::MakeArray {
                        dst,
                        elem_ty,
                        items: lowered_items,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::ArrayRepeat { value, size } => {
                let value = self.compile_expr(func, lowering, value)?;
                let elem_ty = self.infer_operand_type(func, &value);
                let ty = IrType::Array {
                    elem: Box::new(elem_ty.clone()),
                    size: *size,
                };
                let dst = self.builder.push_temp(func, ty);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::MakeArrayRepeat {
                        dst,
                        elem_ty,
                        value,
                        size: *size,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::StructLit { name, fields } => {
                let runtime_name = self.resolve_struct_runtime_name(name);
                let Some((struct_id, struct_fields)) = self.structs.get(&runtime_name).cloned()
                else {
                    self.unsupported(format!("unknown struct `{name}` in IR lowering"));
                    return None;
                };
                let mut ordered = Vec::with_capacity(struct_fields.len());
                for declared in &struct_fields {
                    let Some((_, expr)) = fields
                        .iter()
                        .find(|(field_name, _)| field_name == &declared.name)
                    else {
                        self.unsupported(format!(
                            "missing field `{}` in struct literal `{name}`",
                            declared.name
                        ));
                        return None;
                    };
                    ordered.push(self.compile_expr(func, lowering, expr)?);
                }
                let dst = self
                    .builder
                    .push_temp(func, IrType::Named(runtime_name.clone()));
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::MakeStruct {
                        dst,
                        struct_id,
                        fields: ordered,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::FnLit {
                params,
                return_type,
                body,
            } => self.compile_fn_lit(func, lowering.current_block, params, return_type, body),
            Expr::Index { base, index } => {
                let array = self.compile_expr(func, lowering, base)?;
                let index = self.compile_expr(func, lowering, index)?;
                let elem_ty = self.array_element_type(func, &array);
                let dst = self.builder.push_temp(func, elem_ty.clone());
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::ArrayGet {
                        dst,
                        elem_ty,
                        array,
                        index,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::Group(inner) => self.compile_expr(func, lowering, inner),
            Expr::Unary { op, expr } => {
                let operand = self.compile_expr(func, lowering, expr)?;
                let ty = self.infer_operand_type(func, &operand);
                let dst = self.builder.push_temp(func, ty.clone());
                let op = match op {
                    AstUnaryOp::Neg => UnaryOp::Neg,
                    AstUnaryOp::Not => UnaryOp::Not,
                    AstUnaryOp::Pos => {
                        self.unsupported("unary operator is not in the initial IR lowering subset");
                        return None;
                    }
                };
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::Unary {
                        dst,
                        ty,
                        op,
                        operand,
                    },
                );
                Some(Operand::Temp(dst))
            }
            Expr::Binary { left, op, right } => {
                if matches!(op, AstBinaryOp::AndAnd | AstBinaryOp::OrOr) {
                    return self.compile_short_circuit(func, lowering, left, op, right);
                }
                let left = self.compile_expr(func, lowering, left)?;
                let right = self.compile_expr(func, lowering, right)?;
                let ty = self.infer_binary_type(func, &left, op, &right);
                let dst = self.builder.push_temp(func, ty.clone());
                if let Some(op) = self.lower_binary_op(op) {
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::Binary {
                            dst,
                            ty,
                            op,
                            left,
                            right,
                        },
                    );
                } else if let Some(op) = self.lower_cmp_op(op) {
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::Compare {
                            dst,
                            op,
                            left,
                            right,
                        },
                    );
                } else {
                    self.unsupported("binary operator is not in the initial IR lowering subset");
                    return None;
                }
                Some(Operand::Temp(dst))
            }
            Expr::Call { callee, args } => self.compile_call(func, lowering, callee, args),
            _ => {
                self.unsupported("expression form is not in the initial IR lowering subset");
                None
            }
        }
    }

    fn compile_short_circuit(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        left: &Expr,
        op: &AstBinaryOp,
        right: &Expr,
    ) -> Option<Operand> {
        let left_value = self.compile_expr(func, lowering, left)?;
        let result_local = self.builder.push_local(
            func,
            format!("__sc{}", lowering.scratch_counter),
            IrType::Bool,
        );
        lowering.scratch_counter += 1;

        let rhs_block = self.builder.push_block(func, "sc_rhs");
        let short_block = self.builder.push_block(func, "sc_short");
        let join_block = self.builder.push_block(func, "sc_join");

        let (then_block, else_block, short_value) = match op {
            AstBinaryOp::AndAnd => (rhs_block, short_block, false),
            AstBinaryOp::OrOr => (short_block, rhs_block, true),
            _ => return None,
        };

        self.builder.set_terminator(
            func,
            lowering.current_block,
            Terminator::Branch(BranchTerminator {
                cond: left_value,
                then_block,
                else_block,
            }),
        );

        self.builder.push_instr(
            func,
            short_block,
            Instr::StoreLocal {
                local: result_local,
                ty: IrType::Bool,
                value: Operand::Const(ConstValue::Bool(short_value)),
            },
        );
        self.builder
            .set_terminator(func, short_block, Terminator::Jump(join_block));

        lowering.current_block = rhs_block;
        let right_value = self.compile_expr(func, lowering, right)?;
        self.builder.push_instr(
            func,
            rhs_block,
            Instr::StoreLocal {
                local: result_local,
                ty: IrType::Bool,
                value: right_value,
            },
        );
        self.builder
            .set_terminator(func, rhs_block, Terminator::Jump(join_block));

        lowering.current_block = join_block;
        Some(Operand::Local(result_local))
    }

    fn infer_operand_type(&self, func: &crate::ir::IrFunction, operand: &Operand) -> IrType {
        match operand {
            Operand::Const(ConstValue::Int(_)) => IrType::Int,
            Operand::Const(ConstValue::Float(_)) => IrType::Float,
            Operand::Const(ConstValue::Bool(_)) => IrType::Bool,
            Operand::Const(ConstValue::String(_)) => IrType::String,
            Operand::Const(ConstValue::Unit) => IrType::Void,
            Operand::Temp(id) => func
                .temps
                .iter()
                .find(|temp| temp.id == *id)
                .map(|temp| temp.ty.clone())
                .unwrap_or(IrType::Unknown),
            Operand::Local(id) => func
                .locals
                .iter()
                .find(|local| local.id == *id)
                .map(|local| local.ty.clone())
                .unwrap_or(IrType::Unknown),
            Operand::Global(_) => IrType::Unknown,
        }
    }

    fn array_element_type(&self, func: &crate::ir::IrFunction, operand: &Operand) -> IrType {
        match self.infer_operand_type(func, operand) {
            IrType::Array { elem, .. } => *elem,
            IrType::Vec { elem } => *elem,
            _ => IrType::Unknown,
        }
    }

    fn field_type(&self, func: &crate::ir::IrFunction, base: &Operand, field: &str) -> IrType {
        let IrType::Named(struct_name) = self.infer_operand_type(func, base) else {
            return IrType::Unknown;
        };
        self.structs
            .get(&struct_name)
            .and_then(|(_, fields)| fields.iter().find(|entry| entry.name == field))
            .map(|entry| entry.ty.clone())
            .unwrap_or(IrType::Unknown)
    }

    fn resolve_field_ref(
        &self,
        func: &crate::ir::IrFunction,
        base: &Operand,
        field: &str,
    ) -> crate::ir::FieldRef {
        let index = match self.infer_operand_type(func, base) {
            IrType::Named(struct_name) => self
                .structs
                .get(&struct_name)
                .and_then(|(_, fields)| fields.iter().position(|entry| entry.name == field))
                .unwrap_or(usize::MAX),
            _ => usize::MAX,
        };
        crate::ir::FieldRef {
            index,
            name: field.to_string(),
        }
    }

    fn compile_call(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        callee: &Expr,
        args: &[Expr],
    ) -> Option<Operand> {
        let mut lowered_args = Vec::with_capacity(args.len());
        for arg in args {
            lowered_args.push(self.compile_expr(func, lowering, arg)?);
        }

        match callee {
            Expr::Ident(name) => {
                let direct_name = self
                    .direct_import_calls
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| self.qualify_name(name));
                if let Some((function, ret_ty)) = self.functions.get(&direct_name).cloned() {
                    let dst = if ret_ty.is_void() {
                        None
                    } else {
                        Some(self.builder.push_temp(func, ret_ty.clone()))
                    };
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::CallDirect {
                            dst,
                            ret_ty: ret_ty.clone(),
                            function,
                            args: lowered_args,
                        },
                    );
                    OkOperand::from_call_result(dst)
                } else {
                    let callee = self.compile_expr(func, lowering, callee)?;
                    let dst = self.builder.push_temp(func, IrType::Unknown);
                    self.builder.push_instr(
                        func,
                        lowering.current_block,
                        Instr::CallIndirect {
                            dst: Some(dst),
                            ret_ty: IrType::Unknown,
                            callee,
                            args: lowered_args,
                        },
                    );
                    Some(Operand::Temp(dst))
                }
            }
            Expr::Field { base, field } => {
                if let Expr::Ident(package) = base.as_ref() {
                    let is_value_receiver = lowering.locals.contains_key(package)
                        || self.globals.contains_key(package)
                        || self.globals.contains_key(&self.qualify_name(package))
                        || self.functions.contains_key(package);
                    if package == "vec" {
                        return self.compile_vec_call(
                            func,
                            lowering.current_block,
                            field,
                            lowered_args,
                        );
                    }
                    if !is_value_receiver
                        && let Some(target_name) = self
                            .namespace_call_targets
                            .get(&format!("{package}.{field}"))
                        && let Some((function, ret_ty)) = self.functions.get(target_name).cloned()
                    {
                        let dst = if ret_ty.is_void() {
                            None
                        } else {
                            Some(self.builder.push_temp(func, ret_ty.clone()))
                        };
                        self.builder.push_instr(
                            func,
                            lowering.current_block,
                            Instr::CallDirect {
                                dst,
                                ret_ty: ret_ty.clone(),
                                function,
                                args: lowered_args,
                            },
                        );
                        return OkOperand::from_call_result(dst);
                    }
                    if !is_value_receiver {
                        let dst = self.builder.push_temp(func, IrType::Unknown);
                        self.builder.push_instr(
                            func,
                            lowering.current_block,
                            Instr::CallBuiltin {
                                dst: Some(dst),
                                ret_ty: IrType::Unknown,
                                builtin: crate::ir::BuiltinCall {
                                    package: package.clone(),
                                    name: field.clone(),
                                },
                                args: lowered_args,
                            },
                        );
                        return Some(Operand::Temp(dst));
                    }
                }
                self.compile_method_call(func, lowering, base, field, lowered_args)
            }
            _ => {
                let callee = self.compile_expr(func, lowering, callee)?;
                let dst = self.builder.push_temp(func, IrType::Unknown);
                self.builder.push_instr(
                    func,
                    lowering.current_block,
                    Instr::CallIndirect {
                        dst: Some(dst),
                        ret_ty: IrType::Unknown,
                        callee,
                        args: lowered_args,
                    },
                );
                Some(Operand::Temp(dst))
            }
        }
    }

    fn compile_method_call(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        base: &Expr,
        field: &str,
        mut args: Vec<Operand>,
    ) -> Option<Operand> {
        let receiver = self.compile_expr(func, lowering, base)?;
        let IrType::Named(struct_name) = self.infer_operand_type(func, &receiver) else {
            self.unsupported(
                "method call on non-struct receiver is not in the initial IR lowering subset",
            );
            return None;
        };
        let method_name = Self::mangle_method_name(&struct_name, field);
        let Some((function, ret_ty)) = self.functions.get(&method_name).cloned() else {
            self.unsupported(format!(
                "unknown method `{field}` for struct `{struct_name}` in IR lowering"
            ));
            return None;
        };
        let mut call_args = Vec::with_capacity(args.len() + 1);
        call_args.push(receiver);
        call_args.append(&mut args);
        let dst = if ret_ty.is_void() {
            None
        } else {
            Some(self.builder.push_temp(func, ret_ty.clone()))
        };
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::CallDirect {
                dst,
                ret_ty: ret_ty.clone(),
                function,
                args: call_args,
            },
        );
        OkOperand::from_call_result(dst)
    }

    fn try_compile_vec_new_let(
        &mut self,
        func: &mut crate::ir::IrFunction,
        lowering: &mut FunctionLowering,
        name: &str,
        ty: &Option<crate::ast::TypeName>,
        value: &Expr,
    ) -> Option<bool> {
        let Some(crate::ast::TypeName::Vec { elem }) = ty else {
            return None;
        };
        let Expr::Call { callee, args } = value else {
            return None;
        };
        if !args.is_empty()
            || !matches!(&**callee, Expr::Field { base, field } if field == "new" && matches!(&**base, Expr::Ident(pkg) if pkg == "vec"))
        {
            return None;
        }

        let elem_ty = IrType::from(&TypeInfo::from_ast(elem));
        let local_ty = IrType::Vec {
            elem: Box::new(elem_ty.clone()),
        };
        let local = self
            .builder
            .push_local(func, name.to_string(), local_ty.clone());
        lowering.locals.insert(name.to_string(), local);
        let dst = self.builder.push_temp(func, local_ty.clone());
        self.builder
            .push_instr(func, lowering.current_block, Instr::VecNew { dst, elem_ty });
        self.builder.push_instr(
            func,
            lowering.current_block,
            Instr::StoreLocal {
                local,
                ty: local_ty,
                value: Operand::Temp(dst),
            },
        );
        Some(true)
    }

    fn compile_vec_call(
        &mut self,
        func: &mut crate::ir::IrFunction,
        block: BlockId,
        field: &str,
        args: Vec<Operand>,
    ) -> Option<Operand> {
        match (field, args.as_slice()) {
            ("new", []) => {
                let dst = self.builder.push_temp(
                    func,
                    IrType::Vec {
                        elem: Box::new(IrType::Unknown),
                    },
                );
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecNew {
                        dst,
                        elem_ty: IrType::Unknown,
                    },
                );
                Some(Operand::Temp(dst))
            }
            ("len", [vec]) => {
                let dst = self.builder.push_temp(func, IrType::Int);
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecLen {
                        dst,
                        vec: vec.clone(),
                    },
                );
                Some(Operand::Temp(dst))
            }
            ("push", [vec, value]) => {
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecPush {
                        vec: vec.clone(),
                        value: value.clone(),
                    },
                );
                Some(Operand::Const(ConstValue::Unit))
            }
            ("get", [vec, index]) => {
                let elem_ty = self.array_element_type(func, vec);
                let dst = self.builder.push_temp(func, elem_ty.clone());
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecGet {
                        dst,
                        elem_ty,
                        vec: vec.clone(),
                        index: index.clone(),
                    },
                );
                Some(Operand::Temp(dst))
            }
            ("set", [vec, index, value]) => {
                let elem_ty = self.array_element_type(func, vec);
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecSet {
                        elem_ty,
                        vec: vec.clone(),
                        index: index.clone(),
                        value: value.clone(),
                    },
                );
                Some(Operand::Const(ConstValue::Unit))
            }
            ("delete", [vec, index]) => {
                let elem_ty = self.array_element_type(func, vec);
                let dst = self.builder.push_temp(func, elem_ty.clone());
                self.builder.push_instr(
                    func,
                    block,
                    Instr::VecDelete {
                        dst,
                        elem_ty,
                        vec: vec.clone(),
                        index: index.clone(),
                    },
                );
                Some(Operand::Temp(dst))
            }
            _ => {
                self.unsupported(format!("vec.{field} is not supported in IR lowering"));
                None
            }
        }
    }

    fn infer_binary_type(
        &self,
        func: &crate::ir::IrFunction,
        left: &Operand,
        op: &AstBinaryOp,
        right: &Operand,
    ) -> IrType {
        if self.lower_cmp_op(op).is_some() {
            IrType::Bool
        } else {
            let left_ty = self.infer_operand_type(func, left);
            let right_ty = self.infer_operand_type(func, right);
            if left_ty == right_ty {
                left_ty
            } else {
                IrType::Unknown
            }
        }
    }

    fn lower_binary_op(&self, op: &AstBinaryOp) -> Option<BinaryOp> {
        match op {
            AstBinaryOp::Add => Some(BinaryOp::Add),
            AstBinaryOp::Sub => Some(BinaryOp::Sub),
            AstBinaryOp::Mul => Some(BinaryOp::Mul),
            AstBinaryOp::Div => Some(BinaryOp::Div),
            AstBinaryOp::Mod => Some(BinaryOp::Mod),
            AstBinaryOp::AndAnd | AstBinaryOp::OrOr => None,
            _ => None,
        }
    }

    fn lower_cmp_op(&self, op: &AstBinaryOp) -> Option<crate::ir::CmpOp> {
        match op {
            AstBinaryOp::EqEq => Some(crate::ir::CmpOp::Eq),
            AstBinaryOp::Neq => Some(crate::ir::CmpOp::Ne),
            AstBinaryOp::Lt => Some(crate::ir::CmpOp::Lt),
            AstBinaryOp::Lte => Some(crate::ir::CmpOp::Le),
            AstBinaryOp::Gt => Some(crate::ir::CmpOp::Gt),
            AstBinaryOp::Gte => Some(crate::ir::CmpOp::Ge),
            _ => None,
        }
    }

    fn ensure_fallthrough_jump(
        &self,
        func: &mut crate::ir::IrFunction,
        from: BlockId,
        to: BlockId,
    ) {
        if matches!(
            func.blocks
                .iter()
                .find(|block| block.id == from)
                .map(|block| &block.terminator),
            Some(Terminator::Unreachable)
        ) {
            self.builder
                .set_terminator(func, from, Terminator::Jump(to));
        }
    }

    fn unsupported(&mut self, message: impl Into<String>) {
        self.diags.error(message, Span::default());
    }

    fn function_value(
        &mut self,
        func: &mut crate::ir::IrFunction,
        block: BlockId,
        name: &str,
    ) -> Option<Operand> {
        let (function, ret_ty) = self.functions.get(name).cloned()?;
        let dst = self.builder.push_temp(
            func,
            IrType::Fn {
                params: Vec::new(),
                ret: Box::new(ret_ty),
            },
        );
        self.builder
            .push_instr(func, block, Instr::MakeClosure { dst, function });
        Some(Operand::Temp(dst))
    }

    fn compile_fn_lit(
        &mut self,
        outer_func: &mut crate::ir::IrFunction,
        block: BlockId,
        params: &[crate::ast::Param],
        return_type: &crate::ast::TypeName,
        body: &[Stmt],
    ) -> Option<Operand> {
        self.fn_lit_counter += 1;
        let name = format!("__fn_lit_{}", self.fn_lit_counter);
        let ret_ty = IrType::from(&TypeInfo::from_ast(return_type));
        let function_id = crate::ir::FunctionId(self.functions.len() + self.lifted_functions.len());
        self.functions
            .insert(name.clone(), (function_id, ret_ty.clone()));

        let mut lifted = self.builder.begin_function(name, ret_ty.clone());
        lifted.id = function_id;
        let mut lowering = FunctionLowering {
            current_block: lifted.entry,
            locals: HashMap::new(),
            scratch_counter: 0,
        };
        let mut param_types = Vec::with_capacity(params.len());
        for param in params {
            let ty_info = TypeInfo::from_ast(&param.ty);
            let ir_ty = IrType::from(&ty_info);
            param_types.push(ir_ty.clone());
            self.builder
                .push_param(&mut lifted, param.name.clone(), ir_ty.clone());
            let local = self
                .builder
                .push_local(&mut lifted, param.name.clone(), ir_ty);
            lowering.locals.insert(param.name.clone(), local);
        }
        for stmt in body {
            if !self.compile_stmt(&mut lifted, &mut lowering, stmt) {
                return None;
            }
        }
        if matches!(
            lifted
                .blocks
                .iter()
                .find(|block| block.id == lowering.current_block)
                .map(|block| &block.terminator),
            Some(Terminator::Unreachable)
        ) {
            let terminator = if ret_ty.is_void() {
                Terminator::Return(None)
            } else {
                self.diags.error(
                    "IR lowering currently requires explicit return in non-void function literal",
                    Span::default(),
                );
                return None;
            };
            self.builder
                .set_terminator(&mut lifted, lowering.current_block, terminator);
        }
        self.lifted_functions.push(lifted);

        let dst = self.builder.push_temp(
            outer_func,
            IrType::Fn {
                params: param_types,
                ret: Box::new(ret_ty),
            },
        );
        self.builder.push_instr(
            outer_func,
            block,
            Instr::MakeClosure {
                dst,
                function: function_id,
            },
        );
        Some(Operand::Temp(dst))
    }

    fn mangle_method_name(target: &str, method: &str) -> String {
        format!("{target}::{method}")
    }
}

struct OkOperand;

impl OkOperand {
    fn from_call_result(dst: Option<crate::ir::TempId>) -> Option<Operand> {
        Some(match dst {
            Some(dst) => Operand::Temp(dst),
            None => Operand::Const(ConstValue::Unit),
        })
    }
}
