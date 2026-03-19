use skepabench::cli::parse_args;

#[test]
fn parse_args_accepts_valid_combinations() {
    let opts = parse_args(
        [
            "--warmups",
            "2",
            "--runs",
            "3",
            "--profile",
            "release",
            "--filter",
            "runtime",
            "--json",
            "--save-baseline",
            "--compare",
            "--baseline-path",
            "x.tsv",
        ]
        .into_iter()
        .map(str::to_string),
    )
    .expect("args should parse");

    assert_eq!(opts.warmups, 2);
    assert_eq!(opts.runs, 3);
    assert_eq!(opts.profile, "release");
    assert_eq!(opts.filter.as_deref(), Some("runtime"));
    assert!(opts.json);
    assert!(opts.save_baseline);
    assert!(opts.compare);
    assert_eq!(
        opts.baseline_path.as_deref(),
        Some(std::path::Path::new("x.tsv"))
    );
}

#[test]
fn parse_args_rejects_zero_and_unknown_values() {
    assert!(parse_args(["--runs", "0"].into_iter().map(str::to_string)).is_err());
    assert!(parse_args(["--profile", "fast"].into_iter().map(str::to_string)).is_err());
    assert!(parse_args(["--wat"].into_iter().map(str::to_string)).is_err());
}
