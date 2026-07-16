mod common;

use skeplib::sema::analyze_project_entry;

fn write_math_module(project: &common::TempProject) {
    project.file(
        "utils/math.sk",
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    );
}

#[test]
fn rejects_direct_import_call_signature_mismatch() {
    let project = common::TempProject::new("direct_import_call_signature_mismatch");
    write_math_module(&project);
    let entry = project.file(
        "main.sk",
        r#"
from utils.math import add;
fn main() -> Int {
  let a = add("x", 1);
  let b = add(1, 2, 3);
  return 0;
}
"#,
    );

    let (res, diags) = analyze_project_entry(&entry).expect("resolver/sema");
    assert!(res.has_errors);
    common::assert_has_diag(&diags, "Argument 1 for `add`: expected Int, got String");
    common::assert_has_diag(&diags, "Arity mismatch for `add`: expected 2, got 3");
}

#[test]
fn rejects_qualified_import_call_signature_mismatch() {
    let project = common::TempProject::new("qualified_import_call_signature_mismatch");
    write_math_module(&project);
    let entry = project.file(
        "main.sk",
        r#"
import utils.math;
fn main() -> Int {
  let a = utils.math.add("x", 1);
  let b = utils.math.add(1, 2, 3);
  return 0;
}
"#,
    );

    let (res, diags) = analyze_project_entry(&entry).expect("resolver/sema");
    assert!(res.has_errors);
    common::assert_has_diag(&diags, "Argument 1 for `add`: expected Int, got String");
    common::assert_has_diag(&diags, "Arity mismatch for `add`: expected 2, got 3");
}

#[test]
fn accepts_file_module_import_qualified_call() {
    let project = common::TempProject::new("file_module_import_qualified_call");
    project.file(
        "lib.sk",
        r#"
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
import lib;
fn main() -> Int { return lib.add(1, 2); }
"#,
    );

    let (res, diags) = analyze_project_entry(&entry).expect("resolver/sema");
    common::assert_sema_success(&res, &diags);
}

#[test]
fn rejects_folder_import_call_missing_file_segment() {
    let project = common::TempProject::new("folder_import_missing_file_segment");
    project.file(
        "string/case.sk",
        r#"
fn up(s: String) -> String { return s; }
export { up };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
import string;
fn main() -> Int {
  let _ = string.up("x");
  return 0;
}
"#,
    );

    let (res, diags) = analyze_project_entry(&entry).expect("resolver/sema");
    assert!(res.has_errors);
    common::assert_has_diag(
        &diags,
        "Invalid namespace call `string.up`: `import string` introduces a folder namespace",
    );
}
