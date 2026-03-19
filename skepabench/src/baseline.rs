use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::{BaselineRecord, BaselineReport, BenchOutcome, BenchRecord, CliOptions, CompareRow};

pub fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

pub fn print_table_report(opts: &CliOptions, results: &[BenchRecord]) {
    println!(
        "skepabench warmups={} runs={} profile={}",
        opts.warmups, opts.runs, opts.profile
    );
    println!(
        "{:<28} {:<8} {:>10} {:>10} {:>10}",
        "case", "kind", "median_ms", "min_ms", "max_ms"
    );
    for result in results {
        match &result.outcome {
            BenchOutcome::Measured(stats) => println!(
                "{:<28} {:<8} {:>10.3} {:>10.3} {:>10.3}",
                result.name,
                result.kind,
                duration_ms(stats.median),
                duration_ms(stats.min),
                duration_ms(stats.max),
            ),
            BenchOutcome::Skipped(reason) => {
                println!(
                    "{:<28} {:<8} skipped    {}",
                    result.name, result.kind, reason
                );
            }
        }
    }
}

pub fn render_json_report(opts: &CliOptions, results: &[BenchRecord]) -> String {
    let report = baseline_report_from_results(opts, results);
    render_report_json(&report)
}

pub fn write_baseline(
    path: &Path,
    opts: &CliOptions,
    results: &[BenchRecord],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let report = baseline_report_from_results(opts, results);
    fs::write(path, render_baseline_tsv(&report)).map_err(|err| err.to_string())
}

pub fn load_baseline(path: &Path) -> Result<BaselineReport, String> {
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    parse_baseline_tsv(&text)
}

pub fn default_baseline_path(profile: &str) -> PathBuf {
    PathBuf::from("skepabench")
        .join("baselines")
        .join(format!("{profile}.tsv"))
}

pub fn compare_results(baseline: &BaselineReport, results: &[BenchRecord]) -> Vec<CompareRow> {
    let mut rows = Vec::new();
    for result in results {
        let BenchOutcome::Measured(stats) = &result.outcome else {
            continue;
        };
        let Some(baseline_record) = baseline
            .results
            .iter()
            .find(|record| record.case == result.name && record.status == "measured")
        else {
            continue;
        };
        let Some(baseline_ms) = baseline_record.median_ms else {
            continue;
        };
        let current_ms = duration_ms(stats.median);
        let delta_ms = current_ms - baseline_ms;
        let delta_pct = if baseline_ms == 0.0 {
            0.0
        } else {
            (delta_ms / baseline_ms) * 100.0
        };
        rows.push(CompareRow {
            case: result.name.to_string(),
            current_ms,
            baseline_ms,
            delta_ms,
            delta_pct,
        });
    }
    rows.sort_by(|a, b| {
        b.delta_pct
            .abs()
            .partial_cmp(&a.delta_pct.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

pub fn print_compare_report(path: &Path, rows: &[CompareRow]) {
    println!("baseline {}", path.display());
    println!(
        "{:<28} {:>12} {:>12} {:>12} {:>10}",
        "case", "current_ms", "base_ms", "delta_ms", "delta_pct"
    );
    for row in rows {
        println!(
            "{:<28} {:>12.3} {:>12.3} {:>12.3} {:>9.1}%",
            row.case, row.current_ms, row.baseline_ms, row.delta_ms, row.delta_pct
        );
    }
}

pub fn render_compare_json(path: &Path, rows: &[CompareRow]) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"baseline_path\": \"{}\",\n",
        json_escape(&path.display().to_string())
    ));
    out.push_str("  \"rows\": [\n");
    for (idx, row) in rows.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!(
            "      \"case\": \"{}\",\n",
            json_escape(&row.case)
        ));
        out.push_str(&format!("      \"current_ms\": {:.3},\n", row.current_ms));
        out.push_str(&format!("      \"baseline_ms\": {:.3},\n", row.baseline_ms));
        out.push_str(&format!("      \"delta_ms\": {:.3},\n", row.delta_ms));
        out.push_str(&format!("      \"delta_pct\": {:.3}\n", row.delta_pct));
        out.push_str("    }");
        if idx + 1 != rows.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}");
    out
}

pub fn baseline_report_from_results(opts: &CliOptions, results: &[BenchRecord]) -> BaselineReport {
    BaselineReport {
        warmups: opts.warmups,
        runs: opts.runs,
        profile: opts.profile.clone(),
        results: results
            .iter()
            .map(|result| match &result.outcome {
                BenchOutcome::Measured(stats) => BaselineRecord {
                    case: result.name.to_string(),
                    kind: result.kind.to_string(),
                    status: "measured".to_string(),
                    median_ms: Some(duration_ms(stats.median)),
                    min_ms: Some(duration_ms(stats.min)),
                    max_ms: Some(duration_ms(stats.max)),
                    reason: None,
                },
                BenchOutcome::Skipped(reason) => BaselineRecord {
                    case: result.name.to_string(),
                    kind: result.kind.to_string(),
                    status: "skipped".to_string(),
                    median_ms: None,
                    min_ms: None,
                    max_ms: None,
                    reason: Some(reason.clone()),
                },
            })
            .collect(),
    }
}

fn render_report_json(report: &BaselineReport) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"warmups\": {},\n", report.warmups));
    out.push_str(&format!("  \"runs\": {},\n", report.runs));
    out.push_str(&format!(
        "  \"profile\": \"{}\",\n",
        json_escape(&report.profile)
    ));
    out.push_str("  \"results\": [\n");
    for (idx, record) in report.results.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!(
            "      \"case\": \"{}\",\n",
            json_escape(&record.case)
        ));
        out.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&record.kind)
        ));
        out.push_str(&format!(
            "      \"status\": \"{}\"",
            json_escape(&record.status)
        ));
        if let Some(median_ms) = record.median_ms {
            out.push_str(",\n");
            out.push_str(&format!("      \"median_ms\": {:.3},\n", median_ms));
            out.push_str(&format!(
                "      \"min_ms\": {:.3},\n",
                record.min_ms.unwrap_or_default()
            ));
            out.push_str(&format!(
                "      \"max_ms\": {:.3}\n",
                record.max_ms.unwrap_or_default()
            ));
        } else if let Some(reason) = &record.reason {
            out.push_str(",\n");
            out.push_str(&format!("      \"reason\": \"{}\"\n", json_escape(reason)));
        } else {
            out.push('\n');
        }
        out.push_str("    }");
        if idx + 1 != report.results.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}");
    out
}

fn render_baseline_tsv(report: &BaselineReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("warmups\t{}\n", report.warmups));
    out.push_str(&format!("runs\t{}\n", report.runs));
    out.push_str(&format!("profile\t{}\n", escape_tsv(&report.profile)));
    out.push_str("case\tkind\tstatus\tmedian_ms\tmin_ms\tmax_ms\treason\n");
    for record in &report.results {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            escape_tsv(&record.case),
            escape_tsv(&record.kind),
            escape_tsv(&record.status),
            record
                .median_ms
                .map(|v| format!("{v:.3}"))
                .unwrap_or_default(),
            record.min_ms.map(|v| format!("{v:.3}")).unwrap_or_default(),
            record.max_ms.map(|v| format!("{v:.3}")).unwrap_or_default(),
            escape_tsv(record.reason.as_deref().unwrap_or_default()),
        ));
    }
    out
}

fn parse_baseline_tsv(text: &str) -> Result<BaselineReport, String> {
    let mut lines = text.lines();
    let warmups = parse_header_usize(lines.next(), "warmups")?;
    let runs = parse_header_usize(lines.next(), "runs")?;
    let profile = parse_header_string(lines.next(), "profile")?;
    let Some(header) = lines.next() else {
        return Err("baseline file missing results header".to_string());
    };
    if header != "case\tkind\tstatus\tmedian_ms\tmin_ms\tmax_ms\treason" {
        return Err("baseline file has invalid results header".to_string());
    }
    let mut results = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let cols: Vec<_> = line.split('\t').collect();
        if cols.len() != 7 {
            return Err(format!("baseline row has invalid column count: {line}"));
        }
        results.push(BaselineRecord {
            case: unescape_tsv(cols[0])?,
            kind: unescape_tsv(cols[1])?,
            status: unescape_tsv(cols[2])?,
            median_ms: parse_optional_f64(cols[3])?,
            min_ms: parse_optional_f64(cols[4])?,
            max_ms: parse_optional_f64(cols[5])?,
            reason: parse_optional_string(cols[6])?,
        });
    }
    Ok(BaselineReport {
        warmups,
        runs,
        profile,
        results,
    })
}

fn parse_header_usize(line: Option<&str>, key: &str) -> Result<usize, String> {
    let value = parse_header_string(line, key)?;
    value
        .parse::<usize>()
        .map_err(|_| format!("baseline {key} value is not a usize"))
}

fn parse_header_string(line: Option<&str>, key: &str) -> Result<String, String> {
    let Some(line) = line else {
        return Err(format!("baseline file missing `{key}` header"));
    };
    let Some((found, value)) = line.split_once('\t') else {
        return Err(format!("baseline `{key}` header is malformed"));
    };
    if found != key {
        return Err(format!("baseline header order mismatch: expected `{key}`"));
    }
    unescape_tsv(value)
}

fn parse_optional_f64(raw: &str) -> Result<Option<f64>, String> {
    if raw.is_empty() {
        Ok(None)
    } else {
        raw.parse::<f64>()
            .map(Some)
            .map_err(|_| format!("invalid float value `{raw}` in baseline"))
    }
}

fn parse_optional_string(raw: &str) -> Result<Option<String>, String> {
    if raw.is_empty() {
        Ok(None)
    } else {
        Ok(Some(unescape_tsv(raw)?))
    }
}

fn escape_tsv(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn unescape_tsv(input: &str) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(next) = chars.next() else {
            return Err("invalid trailing escape in baseline".to_string());
        };
        match next {
            '\\' => out.push('\\'),
            't' => out.push('\t'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            other => return Err(format!("invalid escape `\\{other}` in baseline")),
        }
    }
    Ok(out)
}

fn json_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
            c => escaped.push(c),
        }
    }
    escaped
}
