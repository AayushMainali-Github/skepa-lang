use std::path::PathBuf;
use std::process::Command;

#[test]
fn examples_run_successfully() {
    run_example("hello.sk", 0);
    run_example("sum_loop.sk", 10);
    run_example("float_math.sk", 0);
}

fn run_example(name: &str, expected_code: i32) {
    let path = repo_root().join("examples").join(name);
    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&path)
        .output()
        .expect("run skeparun");
    assert_eq!(
        output.status.code(),
        Some(expected_code),
        "example {} failed\nstdout:\n{}\nstderr:\n{}",
        name,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn skeparun_bin() -> &'static str {
    env!("CARGO_BIN_EXE_skeparun")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}
