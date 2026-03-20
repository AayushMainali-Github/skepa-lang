use std::path::Path;
use std::process::Command;
use std::time::Duration;

use skepabench::{
    BenchOutcome, BenchRecord, BenchStats, CliOptions, CompareRow, render_full_json_report,
    validate_runtime_output,
};

fn sample_opts() -> CliOptions {
    CliOptions {
        warmups: 1,
        runs: 1,
        profile: "debug".into(),
        filter: None,
        json: true,
        save_baseline: false,
        compare: false,
        baseline_path: None,
    }
}

fn sample_results() -> Vec<BenchRecord> {
    vec![BenchRecord {
        name: "runtime_x",
        kind: "lib",
        outcome: BenchOutcome::Measured(BenchStats {
            min: Duration::from_millis(1),
            median: Duration::from_millis(2),
            max: Duration::from_millis(3),
        }),
    }]
}

#[test]
fn json_compare_renders_single_document() {
    let opts = sample_opts();
    let results = sample_results();
    let rows = vec![CompareRow {
        case: "runtime_x".into(),
        current_ms: 2.0,
        baseline_ms: 1.0,
        delta_ms: 1.0,
        delta_pct: 100.0,
    }];
    let rendered = render_full_json_report(&opts, &results, Some((Path::new("base.tsv"), &rows)));
    assert!(rendered.contains("\"results\": ["));
    assert!(rendered.contains("\"compare\": {"));
    assert_eq!(rendered.matches("\"baseline_path\"").count(), 1);
}

#[test]
fn runtime_command_validation_allows_nonzero_exit_without_runtime_error_output() {
    let output = if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "exit", "7"])
            .output()
            .expect("cmd output")
    } else {
        Command::new("sh")
            .args(["-c", "exit 7"])
            .output()
            .expect("sh output")
    };
    validate_runtime_output(Path::new("tool"), &["run", "x.sk"], &output)
        .expect("nonzero exit without stderr should be treated as benchmark success");
}

#[test]
fn runtime_command_validation_rejects_nonzero_exit_with_runtime_error_output() {
    let output = if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "echo runtime failed 1>&2 && exit 7"])
            .output()
            .expect("cmd output")
    } else {
        Command::new("sh")
            .args(["-c", "echo runtime failed 1>&2; exit 7"])
            .output()
            .expect("sh output")
    };
    let err = validate_runtime_output(Path::new("tool"), &["run", "x.sk"], &output)
        .expect_err("nonzero exit with stderr should fail");
    assert!(err.contains("exited with 7"), "{err}");
}
