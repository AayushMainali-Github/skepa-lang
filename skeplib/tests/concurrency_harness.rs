#[path = "common.rs"]
mod common;

use std::process::Command;
use std::time::Duration;

const CHILD_MODE_ENV: &str = "SKEPA_TEST_PROCESS_CHILD_MODE";

#[test]
fn process_runner_child() {
    match std::env::var(CHILD_MODE_ENV).as_deref() {
        Ok("sleep") => {
            println!("child started");
            std::thread::sleep(Duration::from_secs(5));
        }
        Ok("output") => println!("captured child output"),
        _ => {}
    }
}

fn child_command(mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command
        .args(["--exact", "process_runner_child", "--nocapture"])
        .env(CHILD_MODE_ENV, mode);
    command
}

#[test]
fn shared_process_runner_captures_completed_child_output() {
    let output =
        common::run_output_with_timeout(&mut child_command("output"), Duration::from_secs(5))
            .expect("completed child should return output");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("captured child output"));
}

#[test]
fn shared_process_runner_terminates_a_child_at_its_deadline() {
    let error =
        common::run_output_with_timeout(&mut child_command("sleep"), Duration::from_millis(250))
            .expect_err("hung child should be terminated at the test deadline");

    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
}
