use std::path::Path;

use crate::baseline::{render_compare_json, render_json_report};
use crate::{BenchRecord, CliOptions, CompareRow};

pub fn render_full_json_report(
    opts: &CliOptions,
    results: &[BenchRecord],
    compare: Option<(&Path, &[CompareRow])>,
) -> String {
    let mut out = render_json_report(opts, results);
    if let Some((path, rows)) = compare {
        let compare_json = render_compare_json(path, rows);
        if out.ends_with("\n}") {
            out.truncate(out.len() - 2);
            out.push_str(",\n");
        } else if out.ends_with('}') {
            out.pop();
            out.push_str(",\n");
        } else {
            out.push('\n');
        }
        out.push_str("  \"compare\": ");
        out.push_str(compare_json.trim());
        out.push_str("\n}");
    }
    out
}
