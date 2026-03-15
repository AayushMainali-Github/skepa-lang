use std::collections::HashMap;
use std::path::Path;

use crate::bytecode::{FunctionChunk, StructShape, Value};
use crate::resolver::{ModuleGraph, build_export_maps};

use super::context::Compiler;
use super::peephole::{
    peephole_optimize_module, rewrite_direct_calls_to_indexes, rewrite_function_values_to_indexes,
    rewrite_trivial_direct_calls,
};
use super::{BytecodeModule, Instr};

pub(super) fn compile_project_graph_inner(
    graph: &ModuleGraph,
    entry: &Path,
) -> Result<BytecodeModule, String> {
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

    let mut ids = graph.modules.keys().cloned().collect::<Vec<_>>();
    ids.sort();

    let mut out = BytecodeModule::default();
    let mut linked_method_name_ids = HashMap::new();
    let mut linked_struct_shape_ids = HashMap::new();
    let mut init_names = Vec::new();

    for id in ids {
        let m = &graph.modules[&id];
        let (program, diags) = crate::parser::Parser::parse_source(&m.source);
        if !diags.is_empty() {
            return Err(format!(
                "Parse failed for {}: {:?}",
                m.path.display(),
                diags
            ));
        }
        let mut c = Compiler {
            module_id: Some(id.clone()),
            ..Compiler::default()
        };
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

        let m = c.compile_program(&program);
        let method_id_remap = intern_linked_method_names(
            &mut out.method_names,
            &mut linked_method_name_ids,
            &m.method_names,
        );
        let struct_shape_id_remap = intern_linked_struct_shapes(
            &mut out.struct_shapes,
            &mut linked_struct_shape_ids,
            &m.struct_shapes,
        );
        let init = c.globals_init_name();
        if m.functions.contains_key(&init) {
            init_names.push(init);
        }
        let mut names = m.functions.keys().cloned().collect::<Vec<_>>();
        names.sort();
        for n in names {
            if let Some(mut chunk) = m.functions.get(&n).cloned() {
                remap_chunk_method_ids(&mut chunk, &method_id_remap);
                remap_chunk_struct_shape_ids(&mut chunk, &struct_shape_id_remap);
                if out.functions.insert(n.clone(), chunk).is_some() {
                    return Err(format!("Duplicate linked function symbol `{n}`"));
                }
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

    peephole_optimize_module(&mut out);
    rewrite_direct_calls_to_indexes(&mut out);
    rewrite_trivial_direct_calls(&mut out);
    rewrite_function_values_to_indexes(&mut out);
    Ok(out)
}

fn intern_linked_method_names(
    out_method_names: &mut Vec<String>,
    linked_method_name_ids: &mut HashMap<String, usize>,
    module_method_names: &[String],
) -> Vec<usize> {
    let mut remap = Vec::with_capacity(module_method_names.len());
    for name in module_method_names {
        let id = if let Some(id) = linked_method_name_ids.get(name).copied() {
            id
        } else {
            let id = out_method_names.len();
            out_method_names.push(name.clone());
            linked_method_name_ids.insert(name.clone(), id);
            id
        };
        remap.push(id);
    }
    remap
}

fn intern_linked_struct_shapes(
    out_struct_shapes: &mut Vec<StructShape>,
    linked_struct_shape_ids: &mut HashMap<String, usize>,
    module_struct_shapes: &[StructShape],
) -> Vec<usize> {
    let mut remap = Vec::with_capacity(module_struct_shapes.len());
    for shape in module_struct_shapes {
        let id = if let Some(id) = linked_struct_shape_ids.get(&shape.name).copied() {
            id
        } else {
            let id = out_struct_shapes.len();
            out_struct_shapes.push(shape.clone());
            linked_struct_shape_ids.insert(shape.name.clone(), id);
            id
        };
        remap.push(id);
    }
    remap
}

fn remap_chunk_method_ids(chunk: &mut FunctionChunk, method_id_remap: &[usize]) {
    for instr in &mut chunk.code {
        if let Instr::CallMethodId { id, .. } = instr
            && let Some(mapped) = method_id_remap.get(*id).copied()
        {
            *id = mapped;
        }
    }
}

fn remap_chunk_struct_shape_ids(chunk: &mut FunctionChunk, struct_shape_id_remap: &[usize]) {
    for instr in &mut chunk.code {
        if let Instr::MakeStructId { id } = instr
            && let Some(mapped) = struct_shape_id_remap.get(*id).copied()
        {
            *id = mapped;
        }
    }
}
