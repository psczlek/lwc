mod lwc;

use std::process::ExitCode;

use clap::Parser;
use colored::Colorize;

fn main() -> ExitCode {
    let args = lwc::Args::parse();

    colored::control::set_override(args.colors);

    let result = lwc::count(args);
    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {e}", "lwc".red());
            ExitCode::FAILURE
        }
    }
}
