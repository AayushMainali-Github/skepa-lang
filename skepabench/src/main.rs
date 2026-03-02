use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use skeplib::bytecode::{BytecodeModule, compile_project_graph, compile_source};
use skeplib::diagnostic::DiagnosticBag;
use skeplib::parser::Parser;
use skeplib::resolver::resolve_project;
use skeplib::sema::analyze_project_graph_phased;
use skeplib::vm::Vm;

const DEFAULT_WARMUPS: usize = 2;
const DEFAULT_RUNS: usize = 9;

struct CliOptions {
    warmups: usize,
    runs: usize,
    profile: String,
    filter: Option<String>,
    json: bool,
}

enum CaseKind {
    Library,
    Cli,
}

struct BenchCase {
    name: &'static str,
    kind: CaseKind,
    runner: Box<dyn FnMut() -> Result<(), String>>,
}

struct BenchStats {
    min: Duration,
    median: Duration,
    max: Duration,
}

enum BenchOutcome {
    Measured(BenchStats),
    Skipped(String),
}

struct BenchRecord {
    name: &'static str,
    kind: &'static str,
    outcome: BenchOutcome,
}

struct BenchWorkspace {
    root: PathBuf,
    small_file: PathBuf,
    medium_entry: PathBuf,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let opts = parse_args(env::args().skip(1))?;
    let workspace = BenchWorkspace::create().map_err(|err| err.to_string())?;
    let mut cases = benchmark_cases(&workspace, &opts)?;

    let mut results = Vec::new();

    for case in &mut cases {
        if let Some(filter) = &opts.filter
            && !case.name.contains(filter)
        {
            continue;
        }
        match measure_case(case, opts.warmups, opts.runs) {
            Ok(outcome) => results.push(BenchRecord {
                name: case.name,
                kind: case_kind_label(&case.kind),
                outcome,
            }),
            Err(err) => return Err(format!("benchmark `{}` failed: {err}", case.name)),
        }
    }

    if opts.json {
        println!("{}", render_json_report(&opts, &results));
    } else {
        print_table_report(&opts, &results);
    }

    Ok(())
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<CliOptions, String> {
    let mut warmups = DEFAULT_WARMUPS;
    let mut runs = DEFAULT_RUNS;
    let mut profile = String::from("debug");
    let mut filter = None;
    let mut json = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--warmups" => {
                let Some(value) = args.next() else {
                    return Err("Missing value for --warmups".to_string());
                };
                warmups = value
                    .parse::<usize>()
                    .map_err(|_| "--warmups must be a positive integer".to_string())?;
            }
            "--runs" => {
                let Some(value) = args.next() else {
                    return Err("Missing value for --runs".to_string());
                };
                runs = value
                    .parse::<usize>()
                    .map_err(|_| "--runs must be a positive integer".to_string())?;
            }
            "--profile" => {
                let Some(value) = args.next() else {
                    return Err("Missing value for --profile".to_string());
                };
                if value != "debug" && value != "release" {
                    return Err("--profile must be `debug` or `release`".to_string());
                }
                profile = value;
            }
            "--filter" => {
                let Some(value) = args.next() else {
                    return Err("Missing value for --filter".to_string());
                };
                filter = Some(value);
            }
            "--json" => {
                json = true;
            }
            "--help" | "-h" => {
                return Err(
                    "Usage: cargo run -p skepabench -- [--warmups N] [--runs N] [--profile debug|release] [--filter SUBSTR] [--json]"
                        .to_string(),
                );
            }
            _ => return Err(format!("Unknown argument `{arg}`")),
        }
    }

    if warmups == 0 || runs == 0 {
        return Err("--warmups and --runs must be >= 1".to_string());
    }

    Ok(CliOptions {
        warmups,
        runs,
        profile,
        filter,
        json,
    })
}

fn benchmark_cases(
    workspace: &BenchWorkspace,
    opts: &CliOptions,
) -> Result<Vec<BenchCase>, String> {
    let small_src = fs::read_to_string(&workspace.small_file).map_err(|err| err.to_string())?;
    let medium_graph = resolve_project(&workspace.medium_entry).map_err(format_resolve_errors)?;
    let small_graph = resolve_project(&workspace.small_file).map_err(format_resolve_errors)?;
    let small_graph_for_sema = small_graph.clone();
    let small_graph_for_codegen = small_graph.clone();
    let medium_graph_for_sema = medium_graph.clone();
    let medium_graph_for_codegen = medium_graph.clone();

    let loop_module = compile_source(&src_loop_accumulate(250_000)).map_err(format_diags)?;
    let call_module = compile_source(&src_function_call_chain(120_000)).map_err(format_diags)?;
    let array_module = compile_source(&src_array_workload(100_000)).map_err(format_diags)?;
    let struct_module =
        compile_source(&src_struct_method_workload(60_000)).map_err(format_diags)?;
    let string_module = compile_source(&src_string_workload(15_000)).map_err(format_diags)?;

    let mut cases = vec![
        BenchCase {
            name: "compile_small_parse",
            kind: CaseKind::Library,
            runner: Box::new(move || {
                let _ = Parser::parse_source(&small_src);
                Ok(())
            }),
        },
        BenchCase {
            name: "compile_small_resolve",
            kind: CaseKind::Library,
            runner: Box::new({
                let small_path = workspace.small_file.clone();
                move || {
                    let _ = resolve_project(&small_path).map_err(format_resolve_errors)?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_small_sema",
            kind: CaseKind::Library,
            runner: Box::new(move || {
                let (_result, parse_diags, sema_diags) =
                    analyze_project_graph_phased(&small_graph_for_sema);
                if !parse_diags.is_empty() || !sema_diags.is_empty() {
                    return Err("unexpected diagnostics in compile_small_sema".to_string());
                }
                Ok(())
            }),
        },
        BenchCase {
            name: "compile_small_codegen",
            kind: CaseKind::Library,
            runner: Box::new({
                let small_path = workspace.small_file.clone();
                move || {
                    let _ = compile_project_graph(&small_graph_for_codegen, &small_path)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_medium_resolve",
            kind: CaseKind::Library,
            runner: Box::new({
                let medium_path = workspace.medium_entry.clone();
                move || {
                    let _ = resolve_project(&medium_path).map_err(format_resolve_errors)?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "compile_medium_sema",
            kind: CaseKind::Library,
            runner: Box::new(move || {
                let (_result, parse_diags, sema_diags) =
                    analyze_project_graph_phased(&medium_graph_for_sema);
                if !parse_diags.is_empty() || !sema_diags.is_empty() {
                    return Err("unexpected diagnostics in compile_medium_sema".to_string());
                }
                Ok(())
            }),
        },
        BenchCase {
            name: "compile_medium_codegen",
            kind: CaseKind::Library,
            runner: Box::new({
                let medium_path = workspace.medium_entry.clone();
                move || {
                    let _ = compile_project_graph(&medium_graph_for_codegen, &medium_path)
                        .map_err(|err| err.to_string())?;
                    Ok(())
                }
            }),
        },
        BenchCase {
            name: "runtime_loop_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&loop_module)),
        },
        BenchCase {
            name: "runtime_call_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&call_module)),
        },
        BenchCase {
            name: "runtime_array_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&array_module)),
        },
        BenchCase {
            name: "runtime_struct_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&struct_module)),
        },
        BenchCase {
            name: "runtime_string_heavy",
            kind: CaseKind::Library,
            runner: Box::new(move || run_module(&string_module)),
        },
    ];

    let cli_tools = cli_tools(&opts.profile)?;
    if let Some((skepac, skeparun)) = cli_tools {
        let skepac_small = skepac.clone();
        cases.push(BenchCase {
            name: "cli_small_check",
            kind: CaseKind::Cli,
            runner: Box::new({
                let small_path = workspace.small_file.clone();
                move || run_command(&skepac_small, &["check", path_str(&small_path)?])
            }),
        });
        let skeparun_small = skeparun.clone();
        cases.push(BenchCase {
            name: "cli_small_run",
            kind: CaseKind::Cli,
            runner: Box::new({
                let small_path = workspace.small_file.clone();
                move || run_command(&skeparun_small, &["run", path_str(&small_path)?])
            }),
        });
        let skepac_medium = skepac.clone();
        cases.push(BenchCase {
            name: "cli_medium_check",
            kind: CaseKind::Cli,
            runner: Box::new({
                let medium_path = workspace.medium_entry.clone();
                move || run_command(&skepac_medium, &["check", path_str(&medium_path)?])
            }),
        });
        let skeparun_medium = skeparun.clone();
        cases.push(BenchCase {
            name: "cli_medium_run",
            kind: CaseKind::Cli,
            runner: Box::new({
                let medium_path = workspace.medium_entry.clone();
                move || run_command(&skeparun_medium, &["run", path_str(&medium_path)?])
            }),
        });
    } else {
        cases.push(skipped_case(
            "cli_small_check",
            "missing skepac/skeparun binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_small_run",
            "missing skepac/skeparun binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_medium_check",
            "missing skepac/skeparun binary in selected profile",
        ));
        cases.push(skipped_case(
            "cli_medium_run",
            "missing skepac/skeparun binary in selected profile",
        ));
    }

    Ok(cases)
}

fn skipped_case(name: &'static str, reason: &'static str) -> BenchCase {
    BenchCase {
        name,
        kind: CaseKind::Cli,
        runner: Box::new(move || Err(format!("SKIP:{reason}"))),
    }
}

fn cli_tools(profile: &str) -> Result<Option<(PathBuf, PathBuf)>, String> {
    let exe_dir = env::current_exe()
        .map_err(|err| err.to_string())?
        .parent()
        .ok_or_else(|| "failed to locate current executable directory".to_string())?
        .to_path_buf();

    let expected_profile = exe_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_default();
    if expected_profile != profile {
        return Ok(None);
    }

    let skepac = exe_dir.join(exe_name("skepac"));
    let skeparun = exe_dir.join(exe_name("skeparun"));
    if skepac.exists() && skeparun.exists() {
        Ok(Some((skepac, skeparun)))
    } else {
        Ok(None)
    }
}

fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn measure_case(case: &mut BenchCase, warmups: usize, runs: usize) -> Result<BenchOutcome, String> {
    for _ in 0..warmups {
        match (case.runner)() {
            Ok(()) => {}
            Err(err) => {
                if let Some(reason) = err.strip_prefix("SKIP:") {
                    return Ok(BenchOutcome::Skipped(reason.to_string()));
                }
                return Err(err);
            }
        }
    }

    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let started = Instant::now();
        match (case.runner)() {
            Ok(()) => samples.push(started.elapsed()),
            Err(err) => {
                if let Some(reason) = err.strip_prefix("SKIP:") {
                    return Ok(BenchOutcome::Skipped(reason.to_string()));
                }
                return Err(err);
            }
        }
    }

    samples.sort();
    let min = samples[0];
    let max = samples[samples.len() - 1];
    let median = samples[samples.len() / 2];
    Ok(BenchOutcome::Measured(BenchStats { min, median, max }))
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn print_table_report(opts: &CliOptions, results: &[BenchRecord]) {
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
            BenchOutcome::Measured(stats) => {
                println!(
                    "{:<28} {:<8} {:>10.3} {:>10.3} {:>10.3}",
                    result.name,
                    result.kind,
                    duration_ms(stats.median),
                    duration_ms(stats.min),
                    duration_ms(stats.max),
                );
            }
            BenchOutcome::Skipped(reason) => {
                println!(
                    "{:<28} {:<8} skipped    {}",
                    result.name, result.kind, reason
                );
            }
        }
    }
}

fn render_json_report(opts: &CliOptions, results: &[BenchRecord]) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"warmups\": {},\n", opts.warmups));
    out.push_str(&format!("  \"runs\": {},\n", opts.runs));
    out.push_str(&format!(
        "  \"profile\": \"{}\",\n",
        json_escape(&opts.profile)
    ));
    out.push_str("  \"results\": [\n");

    for (idx, result) in results.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!(
            "      \"case\": \"{}\",\n",
            json_escape(result.name)
        ));
        out.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(result.kind)
        ));
        match &result.outcome {
            BenchOutcome::Measured(stats) => {
                out.push_str("      \"status\": \"measured\",\n");
                out.push_str(&format!(
                    "      \"median_ms\": {:.3},\n",
                    duration_ms(stats.median)
                ));
                out.push_str(&format!(
                    "      \"min_ms\": {:.3},\n",
                    duration_ms(stats.min)
                ));
                out.push_str(&format!(
                    "      \"max_ms\": {:.3}\n",
                    duration_ms(stats.max)
                ));
            }
            BenchOutcome::Skipped(reason) => {
                out.push_str("      \"status\": \"skipped\",\n");
                out.push_str(&format!("      \"reason\": \"{}\"\n", json_escape(reason)));
            }
        }
        out.push_str("    }");
        if idx + 1 != results.len() {
            out.push(',');
        }
        out.push('\n');
    }

    out.push_str("  ]\n");
    out.push('}');
    out
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

fn case_kind_label(kind: &CaseKind) -> &'static str {
    match kind {
        CaseKind::Library => "lib",
        CaseKind::Cli => "cli",
    }
}

fn run_module(module: &BytecodeModule) -> Result<(), String> {
    match Vm::run_module_main(module) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

fn run_command(exe: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new(exe)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run {}: {err}", exe.display()))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "{} {} failed with {}: {}",
            exe.display(),
            args.join(" "),
            output.status,
            stderr.trim()
        ))
    }
}

fn path_str(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| format!("non-utf8 path: {}", path.display()))
}

fn format_resolve_errors(errs: Vec<skeplib::resolver::ResolveError>) -> String {
    errs.into_iter()
        .map(|err| err.message)
        .collect::<Vec<_>>()
        .join("; ")
}

fn format_diags(diags: DiagnosticBag) -> String {
    diags
        .into_vec()
        .into_iter()
        .map(|diag| diag.message)
        .collect::<Vec<_>>()
        .join("; ")
}

impl BenchWorkspace {
    fn create() -> io::Result<Self> {
        let root = unique_temp_dir("skepabench")?;
        fs::create_dir_all(&root)?;

        let small_file = root.join("small.sk");
        fs::write(&small_file, src_small_single_file())?;

        let medium_entry = root.join("main.sk");
        let math_dir = root.join("utils");
        let model_dir = root.join("models");
        fs::create_dir_all(&math_dir)?;
        fs::create_dir_all(&model_dir)?;
        fs::write(&medium_entry, src_medium_main())?;
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
fn addOne(x: Int) -> Int {
  return x + 1;
}

fn main() -> Int {
  let value = addOne(41);
  if (value == 42) {
    return 0;
  }
  return 1;
}
"#
    .trim()
    .to_string()
}

fn src_medium_main() -> String {
    r#"
from utils.math import accumulate;
from models.user import makeUser;

fn main() -> Int {
  let total = accumulate(2000);
  let u = makeUser(3, "skepa");
  if (u.bump(4) == 7 && total > 0) {
    return 0;
  }
  return 1;
}
"#
    .trim()
    .to_string()
}

fn src_medium_math() -> String {
    r#"
fn accumulate(limit: Int) -> Int {
  let i = 0;
  let acc = 0;
  while (i < limit) {
    acc = acc + i;
    i = i + 1;
  }
  return acc;
}

export { accumulate };
"#
    .trim()
    .to_string()
}

fn src_medium_user() -> String {
    r#"
struct User { id: Int, name: String }

impl User {
  fn bump(self, delta: Int) -> Int {
    return self.id + delta;
  }
}

fn makeUser(id: Int, name: String) -> User {
  return User { id: id, name: name };
}

export { User, makeUser };
"#
    .trim()
    .to_string()
}

fn src_loop_accumulate(iterations: usize) -> String {
    format!(
        r#"
fn main() -> Int {{
  let i = 0;
  let acc = 0;
  while (i < {iterations}) {{
    acc = acc + i;
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

fn src_function_call_chain(iterations: usize) -> String {
    format!(
        r#"
fn step(x: Int) -> Int {{
  return x + 1;
}}

fn main() -> Int {{
  let i = 0;
  while (i < {iterations}) {{
    i = step(i);
  }}
  return i;
}}
"#
    )
}

fn src_array_workload(iterations: usize) -> String {
    format!(
        r#"
fn main() -> Int {{
  let arr: [Int; 8] = [0; 8];
  let i = 0;
  while (i < {iterations}) {{
    let idx = i % 8;
    arr[idx] = arr[idx] + 1;
    i = i + 1;
  }}
  return arr[0] + arr[1] + arr[2] + arr[3] + arr[4] + arr[5] + arr[6] + arr[7];
}}
"#
    )
}

fn src_struct_method_workload(iterations: usize) -> String {
    format!(
        r#"
struct User {{ id: Int }}

impl User {{
  fn bump(self, delta: Int) -> Int {{
    return self.id + delta;
  }}
}}

fn main() -> Int {{
  let u = User {{ id: 1 }};
  let i = 0;
  let acc = 0;
  while (i < {iterations}) {{
    acc = acc + u.bump(2);
    i = i + 1;
  }}
  return acc;
}}
"#
    )
}

fn src_string_workload(iterations: usize) -> String {
    format!(
        r#"
import str;

fn main() -> Int {{
  let i = 0;
  let total = 0;
  while (i < {iterations}) {{
    let s = "skepa-language";
    total = total + str.len(s);
    total = total + str.indexOf(s, "lang");
    let cut = str.slice(s, 0, 5);
    if (str.contains(cut, "ske")) {{
      total = total + 1;
    }}
    i = i + 1;
  }}
  return total;
}}
"#
    )
}
