mod lwc;

use std::io;
use std::process::ExitCode;

use clap::Parser;
use colored::Colorize;

fn main() -> ExitCode {
    let args = lwc::Args::parse();

    let result = run(args);
    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {e}", "error".red().bold());
            ExitCode::FAILURE
        }
    }
}

fn run(args: lwc::Args) -> io::Result<()> {
    let counter = match args.entries {
        Some(_) => match args.dflag {
            false => lwc::Counter::File,
            true => lwc::Counter::Dir,
        },
        None => lwc::Counter::Stdin,
    };
    counter.count(args)
}
