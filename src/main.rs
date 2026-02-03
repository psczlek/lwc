mod command;
mod counter;

use colored::Colorize;
use std::process::ExitCode;

fn main() -> ExitCode {
    match command::run() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {e}", "lwc".red());
            ExitCode::FAILURE
        }
    }
}
