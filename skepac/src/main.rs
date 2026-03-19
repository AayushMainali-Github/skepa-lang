mod cli;
mod commands;
mod output;

fn main() {
    match cli::run() {
        Ok(code) => std::process::exit(code),
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(cli::EXIT_USAGE as i32)
        }
    }
}
