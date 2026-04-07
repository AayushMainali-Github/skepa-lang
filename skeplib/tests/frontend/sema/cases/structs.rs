use super::*;

#[test]
fn sema_rejects_duplicate_struct_declarations() {
    let src = r#"
struct User { id: Int }
struct User { name: String }
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Duplicate struct declaration `User`"))
    );
}

#[test]
fn sema_rejects_duplicate_fields_in_struct() {
    let src = r#"
struct User {
  id: Int,
  id: Int,
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Duplicate field `id` in struct `User`"))
    );
}

#[test]
fn sema_rejects_unknown_type_in_struct_field() {
    let src = r#"
struct User {
  profile: Profile,
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Unknown type in struct `User` field `profile`: `Profile`")
    }));
}

#[test]
fn sema_accepts_struct_field_type_referencing_other_struct() {
    let src = r#"
struct Profile { age: Int }
struct User { profile: Profile }
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_unknown_impl_target() {
    let src = r#"
impl User {
  fn id(self) -> Int { return 1; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Unknown impl target struct `User`"))
    );
}

#[test]
fn sema_rejects_duplicate_methods_in_impl_block() {
    let src = r#"
struct User { id: Int }
impl User {
  fn id(self) -> Int { return 1; }
  fn id(self) -> Int { return 2; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Duplicate method `id` in impl `User`") })
    );
}

#[test]
fn sema_rejects_unknown_type_in_method_signature() {
    let src = r#"
struct User { id: Int }
impl User {
  fn setProfile(self, p: Profile) -> Void { return; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Unknown type in method `setProfile` parameter `p`: `Profile`")
    }));
}

#[test]
fn sema_rejects_duplicate_method_across_multiple_impl_blocks() {
    let src = r#"
struct User { id: Int }
impl User {
  fn id(self) -> Int { return self.id; }
}
impl User {
  fn id(self) -> Int { return 0; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Duplicate method `id` in impl `User`"))
    );
}

#[test]
fn sema_rejects_method_without_self_first_param() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bad(x: Int) -> Int { return x; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Method `User.bad` must declare `self: User` as first parameter")
    }));
}

#[test]
fn sema_rejects_method_with_non_self_first_param_name() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bad(this: User) -> Int { return this.id; }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Method `User.bad` must declare `self: User` as first parameter")
    }));
}

#[test]
fn sema_accepts_struct_literal_field_access_and_field_assignment() {
    let src = r#"
struct User {
  id: Int,
  name: String,
}

fn main() -> Int {
  let u: User = User { id: 7, name: "sam" };
  let v = u.id;
  if (v != 7) {
    return 1;
  }
  u.id = 9;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_struct_literal_unknown_and_missing_fields() {
    let src = r#"
struct User {
  id: Int,
  name: String,
}

fn main() -> Int {
  let _u = User { id: 7, nope: "x" };
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Unknown field `nope` in struct `User` literal")
    }));
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Missing field `name` in struct `User` literal")
    }));
}

#[test]
fn sema_rejects_struct_literal_field_type_mismatch() {
    let src = r#"
struct User { id: Int }
fn main() -> Int {
  let _u = User { id: "x" };
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Type mismatch for field `id` in struct `User` literal")
    }));
}

#[test]
fn sema_rejects_unknown_field_access_and_assignment() {
    let src = r#"
struct User { id: Int }
fn main() -> Int {
  let u = User { id: 1 };
  let _x = u.nope;
  u.nope = 2;
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Unknown field `nope` on struct `User`") })
    );
}

#[test]
fn sema_accepts_struct_method_calls() {
    let src = r#"
struct User { id: Int }
impl User {
  fn add(self, x: Int) -> Int {
    return self.id + x;
  }
}
fn main() -> Int {
  let u = User { id: 7 };
  return u.add(5);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_unknown_struct_method() {
    let src = r#"
struct User { id: Int }
fn main() -> Int {
  let u = User { id: 7 };
  return u.nope(1);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Unknown method `nope` on struct `User`") })
    );
}

#[test]
fn sema_rejects_struct_method_arity_mismatch() {
    let src = r#"
struct User { id: Int }
impl User {
  fn add(self, x: Int) -> Int { return self.id + x; }
}
fn main() -> Int {
  let u = User { id: 7 };
  return u.add();
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Arity mismatch for method `User.add`") })
    );
}

#[test]
fn sema_wrong_arity_method_call_does_not_invent_concrete_return_type() {
    let src = r#"
struct User { id: Int }
impl User {
  fn add(self, x: Int) -> Int { return self.id + x; }
}
fn main() -> Int {
  let u = User { id: 7 };
  let x: Int = u.add();
  return 0;
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(
        &diags,
        "Arity mismatch for method `User.add`: expected 1, got 0",
    );
    assert!(
        !diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Type mismatch in let `x`"))
    );
}

#[test]
fn sema_rejects_struct_method_argument_type_mismatch() {
    let src = r#"
struct User { id: Int }
impl User {
  fn add(self, x: Int) -> Int { return self.id + x; }
}
fn main() -> Int {
  let u = User { id: 7 };
  return u.add("x");
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Argument 1 for method `User.add`") })
    );
}

#[test]
fn sema_rejects_method_call_on_non_struct_value() {
    let src = r#"
fn main() -> Int {
  let x: Int = 1;
  return x.add(2);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Method call requires struct receiver") })
    );
}

#[test]
fn sema_rejects_method_return_type_mismatch() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bad(self) -> Int {
    return "x";
  }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Return type mismatch") })
    );
}

#[test]
fn sema_rejects_method_missing_return_for_non_void() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bad(self) -> Int {
    let x = self.id + 1;
  }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(diags.as_slice().iter().any(|d| {
        d.message
            .contains("Method `User.bad` may exit without returning")
    }));
}

#[test]
fn sema_rejects_unknown_variable_inside_method_body() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bad(self) -> Int {
    return nope;
  }
}
fn main() -> Int { return 0; }
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| { d.message.contains("Unknown variable `nope`") })
    );
}

#[test]
fn sema_accepts_method_using_self_field_in_body() {
    let src = r#"
struct User { id: Int }
impl User {
  fn add(self, delta: Int) -> Int {
    let n: Int = self.id + delta;
    return n;
  }
}
fn main() -> Int {
  let u = User { id: 5 };
  return u.add(3);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_method_call_on_call_expression_receiver() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bump(self, d: Int) -> Int { return self.id + d; }
}
fn makeUser(x: Int) -> User {
  return User { id: x };
}
fn main() -> Int {
  return makeUser(9).bump(4);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_method_call_on_index_expression_receiver() {
    let src = r#"
struct User { id: Int }
impl User {
  fn bump(self, d: Int) -> Int { return self.id + d; }
}
fn main() -> Int {
  let users: [User; 2] = [User { id: 2 }, User { id: 5 }];
  return users[1].bump(7);
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_method_call_on_non_struct_chained_receiver() {
    let src = r#"
fn num() -> Int { return 1; }
fn main() -> Int {
  return num().bump(2);
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert!(
        diags
            .as_slice()
            .iter()
            .any(|d| d.message.contains("Method call requires struct receiver"))
    );
}

#[test]
fn sema_accepts_nested_structs_inside_arrays_and_vecs() {
    let src = r#"
import vec;

struct Profile { score: Int }
struct User { profile: Profile }

fn main() -> Int {
  import option;
  let users: [User; 1] = [User { profile: Profile { score: 7 } }];
  let cache: Vec[User] = vec.new();
  vec.push(cache, users[0]);
  return option.unwrapSome(vec.get(cache, 0)).profile.score;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_accepts_large_struct_and_nested_field_assignment() {
    let src = r#"
struct Profile { score: Int }
struct User {
  id: Int,
  age: Int,
  level: Int,
  points: Int,
  active: Bool,
  name: String,
  profile: Profile,
}

fn main() -> Int {
  let u: User = User {
    id: 1,
    age: 20,
    level: 3,
    points: 9,
    active: true,
    name: "sam",
    profile: Profile { score: 7 },
  };
  u.profile.score = 11;
  return u.profile.score;
}
"#;
    let (result, diags) = analyze_source(src);
    assert_sema_success(&result, &diags);
}

#[test]
fn sema_rejects_invalid_method_in_later_impl_while_allowing_other_methods() {
    let src = r#"
struct User { id: Int }

impl User {
  fn ok(self) -> Int { return self.id; }
}

impl User {
  fn bad(x: Int) -> Int { return x; }
}

fn main() -> Int {
  let u = User { id: 1 };
  return u.ok();
}
"#;
    let (result, diags) = analyze_source(src);
    assert!(result.has_errors);
    assert_has_diag(
        &diags,
        "Method `User.bad` must declare `self: User` as first parameter",
    );
}
