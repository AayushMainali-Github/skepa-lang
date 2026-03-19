use skepabench::workloads::{
    BenchWorkspace, src_array_workload, src_string_workload, src_struct_complex_method_workload,
    src_struct_field_workload,
};
use skeplib::sema::analyze_project_entry;

#[test]
fn workload_sources_cover_expected_runtime_heavy_categories() {
    assert!(src_array_workload(4).contains("arr[idx]"));
    assert!(src_string_workload(4).contains("str.len"));
    assert!(src_struct_field_workload(4).contains("p.a"));
    assert!(src_struct_complex_method_workload(4).contains("fn mix"));
}

#[test]
fn bench_workspace_creates_project_inputs() {
    let workspace = BenchWorkspace::create(32).expect("workspace");
    assert!(workspace.small_file.exists());
    assert!(workspace.medium_entry.exists());
    let small = std::fs::read_to_string(&workspace.small_file).expect("small");
    let medium = std::fs::read_to_string(&workspace.medium_entry).expect("medium");
    assert!(small.contains("fn main()"));
    assert!(medium.contains("makeUser"));
}

#[test]
fn bench_workspace_medium_project_is_sema_clean() {
    let workspace = BenchWorkspace::create(32).expect("workspace");
    let (result, diags) = analyze_project_entry(&workspace.medium_entry).expect("resolver/sema");
    assert!(
        !result.has_errors,
        "unexpected medium workspace sema errors: {:?}",
        diags
    );
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
}
