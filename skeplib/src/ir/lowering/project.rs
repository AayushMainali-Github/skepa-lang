use std::collections::HashSet;
use std::path::Path;

use crate::ir::{Instr, IrProgram, IrType, IrVerifier, Operand, Terminator, opt};
use crate::resolver::{
    ModuleGraph, ResolveError, ResolveErrorKind, build_export_maps, resolve_project,
};

use super::context::{FunctionSig, IrLowerer};

pub fn compile_project_entry(entry: &Path) -> Result<IrProgram, Vec<ResolveError>> {
    let mut ir = compile_project_entry_unoptimized(entry)?;
    opt::optimize_program(&mut ir);
    Ok(ir)
}

pub fn compile_project_entry_unoptimized(entry: &Path) -> Result<IrProgram, Vec<ResolveError>> {
    let graph = resolve_project(entry)?;
    compile_project_graph_unoptimized(&graph, entry).map_err(|e| {
        vec![ResolveError::new(
            ResolveErrorKind::Codegen,
            e,
            Some(entry.to_path_buf()),
        )]
    })
}

pub fn compile_project_graph(graph: &ModuleGraph, entry: &Path) -> Result<IrProgram, String> {
    let mut ir = compile_project_graph_unoptimized(graph, entry)?;
    opt::optimize_program(&mut ir);
    Ok(ir)
}

pub fn compile_project_graph_unoptimized(
    graph: &ModuleGraph,
    entry: &Path,
) -> Result<IrProgram, String> {
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
    let mut init_functions_by_module = std::collections::HashMap::new();
    let mut modules = Vec::new();
    let mut ids = graph.modules.keys().cloned().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        let module = &graph.modules[&id];
        modules.push((id, module.program.clone()));
    }

    for (id, program) in &modules {
        lowerer.configure_project_module(id, program, graph, &export_maps);
        lowerer.register_program_items(program, &mut out);
    }

    for (id, program) in &modules {
        lowerer.configure_project_module(id, program, graph, &export_maps);
        let init_name = lowerer.qualify_name("__globals_init");
        lowerer.lower_program_bodies(program, &mut out);
        if let Some(sig) = lowerer.functions.get(&init_name).cloned()
            && out.functions.iter().any(|func| func.id == sig.id)
        {
            init_functions_by_module.insert(id.clone(), sig.id);
        }
    }

    let init_function_ids = module_init_order(graph, &init_functions_by_module);
    if !init_function_ids.is_empty() {
        let wrapper_id = crate::ir::FunctionId(lowerer.functions.len());
        lowerer.functions.insert(
            "__globals_init".to_string(),
            FunctionSig {
                id: wrapper_id,
                params: Vec::new(),
                ret: IrType::Void,
            },
        );
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
    lowerer.functions.insert(
        "main".to_string(),
        FunctionSig {
            id: wrapper_main_id,
            params: Vec::new(),
            ret: entry_main_ty.clone(),
        },
    );
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

fn module_init_order(
    graph: &ModuleGraph,
    init_functions_by_module: &std::collections::HashMap<String, crate::ir::FunctionId>,
) -> Vec<crate::ir::FunctionId> {
    fn visit(
        id: &str,
        graph: &ModuleGraph,
        init_functions_by_module: &std::collections::HashMap<String, crate::ir::FunctionId>,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        out: &mut Vec<crate::ir::FunctionId>,
    ) {
        if visited.contains(id) || !visiting.insert(id.to_string()) {
            return;
        }
        if let Some(unit) = graph.modules.get(id) {
            let mut deps = unit.imports.clone();
            deps.sort();
            for dep in deps {
                visit(
                    &dep,
                    graph,
                    init_functions_by_module,
                    visiting,
                    visited,
                    out,
                );
            }
        }
        visiting.remove(id);
        visited.insert(id.to_string());
        if let Some(function) = init_functions_by_module.get(id)
            && !out.contains(function)
        {
            out.push(*function);
        }
    }

    let mut out = Vec::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    let mut ids = graph.modules.keys().cloned().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        visit(
            &id,
            graph,
            init_functions_by_module,
            &mut visiting,
            &mut visited,
            &mut out,
        );
    }
    out
}
