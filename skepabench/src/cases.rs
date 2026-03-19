use std::fs;

use skeplib::codegen;
use skeplib::diagnostic::DiagnosticBag;
use skeplib::ir;
use skeplib::parser::Parser;
use skeplib::resolver::resolve_project;
use skeplib::sema::analyze_project_graph_phased;

use crate::process::{
    cli_tools, exe_ext, native_exec_case, object_ext, path_str, run_command, skipped_case,
    temp_artifact_path,
};
use crate::workloads::{
    BenchWorkspace, src_arith_chain_workload, src_arith_local_const_workload,
    src_arith_local_local_workload, src_arith_workload, src_array_workload,
    src_function_call_chain, src_loop_accumulate, src_string_workload,
    src_struct_complex_method_workload, src_struct_field_workload, src_struct_method_workload,
    workload_config,
};
use crate::{BenchCase, CaseKind, CliOptions};

pub fn benchmark_cases(
    workspace: &BenchWorkspace,
    opts: &CliOptions,
) -> Result<Vec<BenchCase>, String> {
    let workloads = workload_config(opts);
    let small_src = fs::read_to_string(&workspace.small_file).map_err(|err| err.to_string())?;
    let small_graph = resolve_project(&workspace.small_file).map_err(format_resolve_errors)?;
    let medium_graph = resolve_project(&workspace.medium_entry).map_err(format_resolve_errors)?;
    let small_graph_for_sema = small_graph.clone();
    let medium_graph_for_sema = medium_graph.clone();

    let loop_src = src_loop_accumulate(workloads.loop_iterations);
    let arith_src = src_arith_workload(workloads.arith_iterations);
    let arith_local_const_src =
        src_arith_local_const_workload(workloads.arith_local_const_iterations);
    let arith_local_local_src =
        src_arith_local_local_workload(workloads.arith_local_local_iterations);
    let arith_chain_src = src_arith_chain_workload(workloads.arith_chain_iterations);
    let call_src = src_function_call_chain(workloads.call_iterations);
    let array_src = src_array_workload(workloads.array_iterations);
    let struct_src = src_struct_method_workload(workloads.struct_iterations);
    let struct_field_src = src_struct_field_workload(workloads.struct_field_iterations);
    let struct_complex_src =
        src_struct_complex_method_workload(workloads.struct_complex_method_iterations);
    let string_src = src_string_workload(workloads.string_iterations);

    let cli_tool = cli_tools(&opts.profile)?;
    let native_exec_cases = if let Some(skepac) = &cli_tool {
        vec![
            native_exec_case("runtime_loop_heavy", skepac.clone(), &loop_src)?,
            native_exec_case("runtime_arith_heavy", skepac.clone(), &arith_src)?,
            native_exec_case(
                "runtime_arith_local_const",
                skepac.clone(),
                &arith_local_const_src,
            )?,
            native_exec_case(
                "runtime_arith_local_local",
                skepac.clone(),
                &arith_local_local_src,
            )?,
            native_exec_case("runtime_arith_chain", skepac.clone(), &arith_chain_src)?,
            native_exec_case("runtime_call_heavy", skepac.clone(), &call_src)?,
            native_exec_case("runtime_array_heavy", skepac.clone(), &array_src)?,
            native_exec_case("runtime_struct_heavy", skepac.clone(), &struct_src)?,
            native_exec_case(
                "runtime_struct_field_heavy",
                skepac.clone(),
                &struct_field_src,
            )?,
            native_exec_case(
                "runtime_struct_method_complex",
                skepac.clone(),
                &struct_complex_src,
            )?,
            native_exec_case("runtime_string_heavy", skepac.clone(), &string_src)?,
        ]
    } else {
        vec![
            skipped_case(
                "runtime_loop_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_arith_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_arith_local_const",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_arith_local_local",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_arith_chain",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_call_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_array_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_struct_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_struct_field_heavy",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_struct_method_complex",
                "missing skepac binary in selected profile",
            ),
            skipped_case(
                "runtime_string_heavy",
                "missing skepac binary in selected profile",
            ),
        ]
    };

    let mut cases = vec![
        BenchCase {
            name: "compile_small_parse",
            kind: CaseKind::Library,
            runner: Box::new(move || {
                let _ = Parser::parse_source(&small_src);
                Ok(())
            }),
        },
        BenchCase {
            name: "compile_small_resolve",
            kind: CaseKind::Library,
            runner: Box::new({
                let small_path = workspace.small_file.clone();
                move || {
                    let _ = resolve_project(&small_path).map_err(format_resolve_errors)?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_small_sema",
            kind: CaseKind::Library,
            runner: Box::new(move || {
                let (_result, parse_diags, sema_diags) =
                    analyze_project_graph_phased(&small_graph_for_sema)
                        .map_err(format_resolve_errors)?;
                if !parse_diags.is_empty() || !sema_diags.is_empty() {
                    return Err("unexpected diagnostics in compile_small_sema".to_string());
                }
                Ok(())
            }),
        },
        BenchCase {
            name: "compile_small_ir_lowering",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = loop_src.clone();
                move || {
                    let _ =
                        ir::lowering::compile_source_unoptimized(&source).map_err(format_diags)?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_small_ir_optimize",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = loop_src.clone();
                move || {
                    let mut program =
                        ir::lowering::compile_source_unoptimized(&source).map_err(format_diags)?;
                    ir::opt::optimize_program(&mut program);
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_small_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = loop_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_small_object",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = loop_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let obj = temp_artifact_path("small_obj", object_ext());
                    let result = codegen::compile_program_to_object_file(&program, &obj)
                        .map_err(|err| err.to_string());
                    let _ = fs::remove_file(&obj);
                    result
                }
            }),
        },
        BenchCase {
            name: "compile_small_link",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = loop_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let obj = temp_artifact_path("small_link_obj", object_ext());
                    let exe = temp_artifact_path("small_link_exe", exe_ext());
                    codegen::compile_program_to_object_file(&program, &obj)
                        .map_err(|err| err.to_string())?;
                    let result = codegen::link_object_file_to_executable(&obj, &exe)
                        .map_err(|err| err.to_string());
                    let _ = fs::remove_file(&obj);
                    let _ = fs::remove_file(&exe);
                    result
                }
            }),
        },
        BenchCase {
            name: "compile_medium_resolve",
            kind: CaseKind::Library,
            runner: Box::new({
                let medium_path = workspace.medium_entry.clone();
                move || {
                    let _ = resolve_project(&medium_path).map_err(format_resolve_errors)?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_medium_sema",
            kind: CaseKind::Library,
            runner: Box::new(move || {
                let (_result, parse_diags, sema_diags) =
                    analyze_project_graph_phased(&medium_graph_for_sema)
                        .map_err(format_resolve_errors)?;
                if !parse_diags.is_empty() || !sema_diags.is_empty() {
                    return Err("unexpected diagnostics in compile_medium_sema".to_string());
                }
                Ok(())
            }),
        },
        BenchCase {
            name: "compile_medium_ir_lowering",
            kind: CaseKind::Library,
            runner: Box::new({
                let entry = workspace.medium_entry.clone();
                move || {
                    let _ = ir::lowering::compile_project_entry_unoptimized(&entry)
                        .map_err(format_resolve_errors)?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_medium_ir_optimize",
            kind: CaseKind::Library,
            runner: Box::new({
                let entry = workspace.medium_entry.clone();
                move || {
                    let mut program = ir::lowering::compile_project_entry_unoptimized(&entry)
                        .map_err(format_resolve_errors)?;
                    ir::opt::optimize_program(&mut program);
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_medium_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let entry = workspace.medium_entry.clone();
                move || {
                    let program = ir::lowering::compile_project_entry(&entry)
                        .map_err(format_resolve_errors)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_array_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = array_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_struct_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = struct_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_struct_field_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = struct_field_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_struct_method_complex_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = struct_complex_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_string_llvm_emit",
            kind: CaseKind::Library,
            runner: Box::new({
                let source = string_src.clone();
                move || {
                    let program = ir::lowering::compile_source(&source).map_err(format_diags)?;
                    let _ = codegen::compile_program_to_llvm_ir(&program)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_medium_object",
            kind: CaseKind::Library,
            runner: Box::new({
                let entry = workspace.medium_entry.clone();
                move || {
                    let program = ir::lowering::compile_project_entry(&entry)
                        .map_err(format_resolve_errors)?;
                    let obj = temp_artifact_path("medium_obj", object_ext());
                    let result = codegen::compile_program_to_object_file(&program, &obj)
                        .map_err(|err| err.to_string());
                    let _ = fs::remove_file(&obj);
                    result
                }
            }),
        },
    ];

    cases.extend(native_exec_cases);

    if let Some(skepac) = cli_tool {
        let skepac_small = skepac.clone();
        cases.push(BenchCase {
            name: "cli_small_check",
            kind: CaseKind::Cli,
            runner: Box::new({
                let skepac_small = skepac_small.clone();
                let small_path = workspace.small_file.clone();
                move || run_command(&skepac_small, &["check", path_str(&small_path)?])
            }),
        });
        cases.push(BenchCase {
            name: "cli_small_run",
            kind: CaseKind::Cli,
            runner: Box::new({
                let skepac_small = skepac_small.clone();
                let small_path = workspace.small_file.clone();
                move || run_command(&skepac_small, &["run", path_str(&small_path)?])
            }),
        });
        let skepac_medium = skepac.clone();
        cases.push(BenchCase {
            name: "cli_medium_check",
            kind: CaseKind::Cli,
            runner: Box::new({
                let skepac_medium = skepac_medium.clone();
                let medium_path = workspace.medium_entry.clone();
                move || run_command(&skepac_medium, &["check", path_str(&medium_path)?])
            }),
        });
        cases.push(BenchCase {
            name: "cli_medium_run",
            kind: CaseKind::Cli,
            runner: Box::new({
                let skepac_medium = skepac_medium.clone();
                let medium_path = workspace.medium_entry.clone();
                move || run_command(&skepac_medium, &["run", path_str(&medium_path)?])
            }),
        });
    } else {
        cases.push(skipped_case(
            "cli_small_check",
            "missing skepac binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_small_run",
            "missing skepac binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_medium_check",
            "missing skepac binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_medium_run",
            "missing skepac binary in selected profile",
        ));
    }

    Ok(cases)
}

fn format_resolve_errors(errs: Vec<skeplib::resolver::ResolveError>) -> String {
    errs.into_iter()
        .map(|err| err.message)
        .collect::<Vec<_>>()
        .join("; ")
}

fn format_diags(diags: DiagnosticBag) -> String {
    diags
        .into_vec()
        .into_iter()
        .map(|diag| diag.message)
        .collect::<Vec<_>>()
        .join("; ")
}
