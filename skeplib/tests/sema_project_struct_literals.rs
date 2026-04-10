mod common;

use skeplib::sema::analyze_project_entry;

#[test]
fn accepts_namespace_aliased_struct_literal() {
    let project = common::TempProject::new("namespace_aliased_struct_literal");
    project.file(
        "models.sk",
        r#"
struct User { id: Int }
export { User };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
import models as m;
fn main() -> Int {
  let u = m.User { id: 7 };
  return u.id;
}
"#,
    );

    let (res, diags) = analyze_project_entry(&entry).expect("resolver/sema");
    common::assert_sema_success(&res, &diags);
}

#[test]
fn reports_field_errors_against_written_struct_literal_name() {
    let project = common::TempProject::new("namespace_aliased_struct_literal_field_error");
    project.file(
        "models.sk",
        r#"
struct User { id: Int }
export { User };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
import models as m;
fn main() -> Int {
  let u: m.User = m.User { name: "ana" };
  return 0;
}
"#,
    );

    let (res, diags) = analyze_project_entry(&entry).expect("resolver/sema");
    assert!(res.has_errors);
    common::assert_has_diag(&diags, "Unknown field `name` in struct `m.User` literal");
    common::assert_has_diag(&diags, "Missing field `id` in struct `m.User` literal");
}
