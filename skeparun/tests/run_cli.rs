use std::process::Command;

#[test]
fn skeparun_reports_deprecation_and_non_zero_exit() {
    let output = Command::new(skeparun_bin())
        .arg("run")
        .arg("anything.sk")
        .output()
        .expect("run skeparun");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("skeparun has been removed; use `skepac run`"));
}

fn skeparun_bin() -> &'static str {
    env!("CARGO_BIN_EXE_skeparun")
}
