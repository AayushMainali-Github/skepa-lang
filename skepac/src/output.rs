use skeplib::diagnostic::Diagnostic;
use skeplib::resolver::ResolveError;

pub fn print_diag(phase: &str, d: &Diagnostic) {
    eprintln!("[{}][{}] {}", phase_code(phase), phase, d);
}

pub fn print_resolve_errors(errs: &[ResolveError]) {
    for e in errs {
        if let Some(path) = &e.path {
            let line = e.line.unwrap_or(0);
            let col = e.col.unwrap_or(0);
            eprintln!(
                "[{}][resolve] {}:{}:{}: {}",
                e.code,
                path.display(),
                line,
                col,
                e.message
            );
        } else {
            eprintln!("[{}][resolve] {}", e.code, e.message);
        }
    }
}

fn phase_code(phase: &str) -> &'static str {
    match phase {
        "parse" => "E-PARSE",
        "sema" => "E-SEMA",
        "codegen" => "E-CODEGEN",
        _ => "E-UNKNOWN",
    }
}
