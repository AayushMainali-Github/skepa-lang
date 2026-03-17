use super::*;

#[test]
fn parses_struct_declaration_with_typed_fields() {
    let src = r#"
struct User {
  id: Int,
  name: String,
}

fn main() -> Int {
  return 0;
}
"#;
    let program = parse_ok(src);
    assert_eq!(program.structs.len(), 1);
    let s = &program.structs[0];
    assert_eq!(s.name, "User");
    assert_eq!(s.fields.len(), 2);
    assert_eq!(s.fields[0].name, "id");
    assert_eq!(s.fields[1].name, "name");
}

#[test]
fn parses_impl_methods_with_self_and_params() {
    let src = r#"
struct User { id: Int, name: String }

impl User {
  fn greet(self) -> String {
    return self.name;
  }

  fn label(self, prefix: String) -> String {
    return prefix + self.name;
  }
}

fn main() -> Int { return 0; }
"#;
    let program = parse_ok(src);
    assert_eq!(program.impls.len(), 1);
    let imp = &program.impls[0];
    assert_eq!(imp.target, "User");
    assert_eq!(imp.methods.len(), 2);
    assert_eq!(imp.methods[0].params[0].name, "self");
    assert_eq!(
        imp.methods[0].params[0].ty,
        TypeName::Named("User".to_string())
    );
    assert_eq!(imp.methods[1].params.len(), 2);
}

#[test]
fn reports_invalid_struct_field_missing_colon() {
    let src = r#"
struct User {
  id Int,
}

fn main() -> Int { return 0; }
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected `:` after field name");
}

#[test]
fn parses_struct_literal_field_access_and_field_assignment_target() {
    let src = r#"
fn main() -> Int {
  let u = User { id: 1, name: "sam" };
  let n = u.name;
  u.name = "max";
  return 0;
}
"#;
    let program = parse_ok(src);
    match &program.functions[0].body[0] {
        Stmt::Let { value, .. } => assert!(matches!(value, Expr::StructLit { .. })),
        _ => panic!("expected struct literal"),
    }
    match &program.functions[0].body[1] {
        Stmt::Let { value, .. } => assert!(matches!(value, Expr::Field { .. })),
        _ => panic!("expected field access"),
    }
    match &program.functions[0].body[2] {
        Stmt::Assign { target, .. } => assert!(matches!(target, AssignTarget::Field { .. })),
        _ => panic!("expected field assignment target"),
    }
}

#[test]
fn parses_vec_type_annotations() {
    let src = r#"
fn take(xs: Vec[Int]) -> Vec[String] {
  let ys: Vec[String] = vec.new();
  return ys;
}
"#;
    let program = parse_ok(src);
    let f = &program.functions[0];
    assert_eq!(f.params[0].ty.as_str(), "Vec[Int]");
    assert_eq!(f.return_type.as_ref().expect("ret").as_str(), "Vec[String]");
    match &f.body[0] {
        Stmt::Let { ty: Some(ty), .. } => assert_eq!(ty.as_str(), "Vec[String]"),
        _ => panic!("expected typed let"),
    }
}

#[test]
fn reports_malformed_vec_type_syntax() {
    let src = r#"
fn main() -> Int {
  let xs: Vec[] = 0;
  return 0;
}
"#;
    let diags = parse_err(src);
    assert_has_diag(&diags, "Expected vector element type");
}

#[test]
fn malformed_struct_literal_recovers_to_following_statements() {
    let src = r#"
struct User {
  id: Int,
  name: String,
}

fn broken() -> Int {
  let bad = User { id: , name: "sam" };
  return 1;
}

fn main() -> Int {
  return 0;
}
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(!diags.is_empty(), "expected diagnostics");
    assert_eq!(program.functions.len(), 2);
    assert_eq!(program.functions[1].name, "main");
}

#[test]
fn malformed_impl_blocks_recover_across_multiple_points() {
    let src = r#"
struct User { id: Int }

impl {
  fn nope(self) -> Int { return 1; }
}

impl User {
  nope(self) -> Int { return 1; }
}

fn main() -> Int { return 0; }
"#;
    let (program, diags) = Parser::parse_source(src);
    assert!(
        diags.len() >= 2,
        "expected multiple diagnostics, got {:?}",
        diags.as_slice()
    );
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "main");
}
