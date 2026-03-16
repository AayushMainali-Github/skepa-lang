use std::process::ExitCode;

const EXIT_DEPRECATED: u8 = 2;

fn main() -> ExitCode {
    eprintln!("skeparun has been removed; use `skepac run`");
    ExitCode::from(EXIT_DEPRECATED)
}
