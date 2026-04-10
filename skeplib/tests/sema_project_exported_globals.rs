mod common;

use skeplib::sema::analyze_project_entry;

#[test]
fn rejects_exported_unannotated_global() {
    let project = common::TempProject::new("exported_unannotated_global");
    let entry = project.file(
        "main.sk",
        r#"
let answer = 42;
export { answer };
fn main() -> Int { return 0; }
"#,
    );

    let (res, diags) = analyze_project_entry(&entry).expect("resolver/sema");
    assert!(res.has_errors);
    common::assert_has_diag(
        &diags,
        "Exported global `answer` must declare an explicit type annotation",
    );
}

#[test]
fn preserves_annotated_global_type_across_modules() {
    let project = common::TempProject::new("annotated_global_type");
    project.file(
        "lib.sk",
        r#"
let answer: Int = 42;
export { answer };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from lib import answer;
fn main() -> Int {
  let wrong: String = answer;
  return 0;
}
"#,
    );

    let (res, diags) = analyze_project_entry(&entry).expect("resolver/sema");
    assert!(res.has_errors);
    common::assert_has_diag(
        &diags,
        "Type mismatch in let `wrong`: declared String, got Int",
    );
}
