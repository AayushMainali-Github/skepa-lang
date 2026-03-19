use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
use skeplib::sema::analyze_project_entry;

use crate::{
    ARITH_CHAIN_ITERATIONS, ARITH_ITERATIONS, ARITH_LOCAL_CONST_ITERATIONS,
    ARITH_LOCAL_LOCAL_ITERATIONS, ARRAY_ITERATIONS, CALL_ITERATIONS, CliOptions, LOOP_ITERATIONS,
    MEDIUM_ACCUMULATE_LIMIT, STRING_ITERATIONS, STRUCT_COMPLEX_METHOD_ITERATIONS,
    STRUCT_FIELD_ITERATIONS, STRUCT_ITERATIONS, WorkloadConfig,
};

pub(crate) struct BenchWorkspace {
    pub root: PathBuf,
    pub small_file: PathBuf,
    pub medium_entry: PathBuf,
}

impl BenchWorkspace {
    pub(crate) fn create(medium_accumulate_limit: usize) -> io::Result<Self> {
        let root = unique_temp_dir("skepabench")?;
        fs::create_dir_all(&root)?;
        let small_file = root.join("small.sk");
        fs::write(&small_file, src_small_single_file())?;
        let medium_entry = root.join("main.sk");
        let math_dir = root.join("utils");
        let model_dir = root.join("models");
        fs::create_dir_all(&math_dir)?;
        fs::create_dir_all(&model_dir)?;
        fs::write(&medium_entry, src_medium_main(medium_accumulate_limit))?;
        fs::write(math_dir.join("math.sk"), src_medium_math())?;
        fs::write(model_dir.join("user.sk"), src_medium_user())?;
        Ok(Self {
            root,
            small_file,
            medium_entry,
        })
    }
}

impl Drop for BenchWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(crate) fn workload_config(_opts: &CliOptions) -> WorkloadConfig {
    WorkloadConfig {
        loop_iterations: LOOP_ITERATIONS,
        arith_iterations: ARITH_ITERATIONS,
        arith_local_const_iterations: ARITH_LOCAL_CONST_ITERATIONS,
        arith_local_local_iterations: ARITH_LOCAL_LOCAL_ITERATIONS,
        arith_chain_iterations: ARITH_CHAIN_ITERATIONS,
        call_iterations: CALL_ITERATIONS,
        array_iterations: ARRAY_ITERATIONS,
        struct_iterations: STRUCT_ITERATIONS,
        struct_field_iterations: STRUCT_FIELD_ITERATIONS,
        struct_complex_method_iterations: STRUCT_COMPLEX_METHOD_ITERATIONS,
        string_iterations: STRING_ITERATIONS,
        medium_accumulate_limit: MEDIUM_ACCUMULATE_LIMIT,
    }
}

fn unique_temp_dir(prefix: &str) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(dir)
}

fn src_small_single_file() -> String {
    r#"
fn addOne(x: Int) -> Int { return x + 1; }
fn main() -> Int {
  let value = addOne(41);
  if (value == 42) { return 0; }
  return 1;
}
"#
    .trim()
    .to_string()
}

fn src_medium_main(medium_accumulate_limit: usize) -> String {
    format!(
        "from utils.math import accumulate;\nfrom models.user import User, makeUser;\n\nfn main() -> Int {{\n  let total = accumulate({medium_accumulate_limit});\n  let u: User = makeUser(3, \"skepa\");\n  if (u.bump(4) == 7 && total > 0) {{ return 0; }}\n  return 1;\n}}"
    )
}

fn src_medium_math() -> String {
    "fn accumulate(limit: Int) -> Int {\n  let i = 0;\n  let acc = 0;\n  while (i < limit) { acc = acc + i; i = i + 1; }\n  return acc;\n}\n\nexport { accumulate };".to_string()
}

fn src_medium_user() -> String {
    "struct User { id: Int, name: String }\n\nimpl User {\n  fn bump(self, delta: Int) -> Int { return self.id + delta; }\n}\n\nfn makeUser(id: Int, name: String) -> User {\n  return User { id: id, name: name };\n}\n\nexport { User, makeUser };".to_string()
}

pub(crate) fn src_loop_accumulate(iterations: usize) -> String {
    format!(
        "fn main() -> Int {{ let i = 0; let acc = 0; while (i < {iterations}) {{ acc = acc + i; i = i + 1; }} return acc; }}"
    )
}
pub(crate) fn src_function_call_chain(iterations: usize) -> String {
    format!(
        "fn step(x: Int) -> Int {{ return x + 1; }}\nfn main() -> Int {{ let i = 0; while (i < {iterations}) {{ i = step(i); }} return i; }}"
    )
}
pub(crate) fn src_arith_workload(iterations: usize) -> String {
    format!(
        "fn main() -> Int {{ let i = 1; let acc = 17; while (i < {iterations}) {{ acc = acc + ((i * 3) % 97); acc = acc - (i % 11); acc = acc + ((acc / 3) % 29); i = i + 1; }} return acc; }}"
    )
}
pub(crate) fn src_arith_local_const_workload(iterations: usize) -> String {
    format!(
        "fn main() -> Int {{ let i = 1; let acc = 17; while (i < {iterations}) {{ let a = acc - 11; let b = a * 3; let c = b / 2; acc = c % 97; i = i + 1; }} return acc; }}"
    )
}
pub(crate) fn src_arith_local_local_workload(iterations: usize) -> String {
    format!(
        "fn main() -> Int {{ let i = 1; let a = 17; let b = 31; let acc = 0; while (i < {iterations}) {{ acc = acc + ((a * b) % 97); acc = acc - (a / b); a = a + 3; b = b + 5; i = i + 1; }} return acc; }}"
    )
}
pub(crate) fn src_arith_chain_workload(iterations: usize) -> String {
    format!(
        "fn main() -> Int {{ let i = 1; let x = 19; let y = 23; let z = 29; let acc = 0; while (i < {iterations}) {{ acc = acc + (((x * 3) + (y * 5) - (z % 7)) / 3); x = x + 1; y = y + 2; z = z + 3; i = i + 1; }} return acc; }}"
    )
}
pub(crate) fn src_array_workload(iterations: usize) -> String {
    format!(
        "fn main() -> Int {{ let arr: [Int; 8] = [0; 8]; let i = 0; while (i < {iterations}) {{ let idx = i % 8; arr[idx] = arr[idx] + 1; i = i + 1; }} return arr[0] + arr[1] + arr[2] + arr[3] + arr[4] + arr[5] + arr[6] + arr[7]; }}"
    )
}
pub(crate) fn src_struct_method_workload(iterations: usize) -> String {
    format!(
        "struct User {{ id: Int }}\nimpl User {{ fn bump(self, delta: Int) -> Int {{ return self.id + delta; }} }}\nfn main() -> Int {{ let u = User {{ id: 1 }}; let i = 0; let acc = 0; while (i < {iterations}) {{ acc = acc + u.bump(2); i = i + 1; }} return acc; }}"
    )
}
pub(crate) fn src_struct_field_workload(iterations: usize) -> String {
    format!(
        "struct Pair {{ a: Int, b: Int }}\nfn main() -> Int {{ let p = Pair {{ a: 11, b: 7 }}; let i = 0; let acc = 0; while (i < {iterations}) {{ acc = acc + p.a; acc = acc + p.b; i = i + 1; }} return acc; }}"
    )
}
pub(crate) fn src_struct_complex_method_workload(iterations: usize) -> String {
    format!(
        "struct Pair {{ a: Int, b: Int }}\nimpl Pair {{ fn mix(self, x: Int) -> Int {{ return ((self.a + x) * 3 + self.b) % 1000000007; }} }}\nfn main() -> Int {{ let p = Pair {{ a: 11, b: 7 }}; let i = 0; let total = 0; while (i < {iterations}) {{ total = total + p.mix(i % 13); i = i + 1; }} return total; }}"
    )
}
pub(crate) fn src_string_workload(iterations: usize) -> String {
    format!(
        "import str;\nfn main() -> Int {{ let i = 0; let total = 0; while (i < {iterations}) {{ let s = \"skepa-language\"; total = total + str.len(s); total = total + str.indexOf(s, \"lang\"); let cut = str.slice(s, 0, 5); if (str.contains(cut, \"ske\")) {{ total = total + 1; }} i = i + 1; }} return total; }}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let (result, diags) =
            analyze_project_entry(&workspace.medium_entry).expect("resolver/sema");
        assert!(
            !result.has_errors,
            "unexpected medium workspace sema errors: {:?}",
            diags
        );
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
    }
}
