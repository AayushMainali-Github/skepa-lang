use std::time::Duration;

use skepabench::baseline::{compare_results, load_baseline, write_baseline};
use skepabench::{
    BaselineRecord, BaselineReport, BenchOutcome, BenchRecord, BenchStats, CliOptions,
};

fn sample_opts() -> CliOptions {
    CliOptions {
        warmups: 1,
        runs: 2,
        profile: "debug".into(),
        filter: None,
        json: false,
        save_baseline: false,
        compare: false,
        baseline_path: None,
    }
}

#[test]
fn baseline_write_and_load_roundtrip_preserves_records() {
    let opts = sample_opts();
    let records = vec![
        BenchRecord {
            name: "runtime_x",
            kind: "lib",
            outcome: BenchOutcome::Measured(BenchStats {
                min: Duration::from_millis(1),
                median: Duration::from_millis(2),
                max: Duration::from_millis(3),
            }),
        },
        BenchRecord {
            name: "runtime_skip",
            kind: "cli",
            outcome: BenchOutcome::Skipped("missing tool".into()),
        },
    ];

    let path = std::env::temp_dir().join("skepabench_roundtrip.tsv");
    write_baseline(&path, &opts, &records).expect("write");
    let loaded = load_baseline(&path).expect("load");
    let _ = std::fs::remove_file(&path);

    assert_eq!(loaded.warmups, 1);
    assert_eq!(loaded.runs, 2);
    assert_eq!(loaded.profile, "debug");
    assert_eq!(loaded.results.len(), 2);
    assert_eq!(loaded.results[0].case, "runtime_x");
    assert_eq!(loaded.results[1].status, "skipped");
}

#[test]
fn compare_results_matches_measured_rows_only() {
    let baseline = BaselineReport {
        warmups: 1,
        runs: 1,
        profile: "debug".into(),
        results: vec![
            BaselineRecord {
                case: "a".into(),
                kind: "lib".into(),
                status: "measured".into(),
                median_ms: Some(10.0),
                min_ms: Some(9.0),
                max_ms: Some(11.0),
                reason: None,
            },
            BaselineRecord {
                case: "b".into(),
                kind: "lib".into(),
                status: "skipped".into(),
                median_ms: None,
                min_ms: None,
                max_ms: None,
                reason: Some("skip".into()),
            },
        ],
    };
    let results = vec![
        BenchRecord {
            name: "a",
            kind: "lib",
            outcome: BenchOutcome::Measured(BenchStats {
                min: Duration::from_millis(5),
                median: Duration::from_millis(20),
                max: Duration::from_millis(25),
            }),
        },
        BenchRecord {
            name: "b",
            kind: "lib",
            outcome: BenchOutcome::Measured(BenchStats {
                min: Duration::from_millis(1),
                median: Duration::from_millis(2),
                max: Duration::from_millis(3),
            }),
        },
    ];

    let rows = compare_results(&baseline, &results);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].case, "a");
    assert_eq!(rows[0].baseline_ms, 10.0);
    assert!(rows[0].delta_pct > 0.0);
}
