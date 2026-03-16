use std::path::PathBuf;
use std::process::Command;

#[test]
fn examples_report_deprecation_through_skeparun() {
    let path = repo_root().join("examples").join("master.sk");
    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg(&path)
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("skeparun has been removed; use `skepac run`"));
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
