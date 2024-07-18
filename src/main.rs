use core::fmt;
use std::env;
use std::fs;
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::process::ExitCode;

const PROG_NAME: &str = "lwc";

macro_rules! warning {
    ($($arg:tt)*) => {{
        if let Err(e) = write!(&mut io::stderr(), "\x1b[1;33mwarning\x1b[0m: {}", format_args!($($arg)*)) {
            eprintln!("Failed to write warning to stderr: {}", e);
        }
    }};
}

macro_rules! error {
    ($($arg:tt)*) => {{
        if let Err(e) = write!(&mut io::stderr(), "\x1b[1;31merror\x1b[0m: {}", format_args!($($arg)*)) {
            eprintln!("Failed to write error to stderr: {}", e);
        }
    }};
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        usage();
        return ExitCode::SUCCESS;
    }

    let mut total_lines = 0;
    let mut total_chars = 0;
    let mut total_words = 0;
    let mut total_bytes = 0;

    for file in args.iter().skip(1) {
        let fhandle = match fs::File::open(file) {
            Ok(f) => {
                let metadata = f.metadata().unwrap();
                let file_type = metadata.file_type();
                if !file_type.is_file() {
                    warning!("'{file}' is not a regular file!\n");
                    continue;
                }
                f
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    warning!("'{file}' not found!\n");
                    continue;
                }
                error!("{e}");
                return ExitCode::FAILURE;
            }
        };

        let counter = Counter::cnt(&fhandle);
        println!("\x1b[1;36m{}\x1b[0m: {}\n", file, counter);

        total_lines += counter.lines;
        total_words += counter.words;
        total_chars += counter.chars;
        total_bytes += counter.bytes;
    }

    println!(
        "\x1b[1;32mTotal\x1b[0m: {} lines, {} words, {} characters, {} bytes",
        total_lines, total_words, total_chars, total_bytes
    );

    ExitCode::SUCCESS
}

#[derive(Debug)]
struct Counter {
    lines: usize,
    chars: usize,
    words: usize,
    bytes: usize,
}

impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.lines == 0 && self.words == 0 && self.chars == 0 && self.bytes == 0 {
            write!(f, "Empty")
        } else {
            write!(
                f,
                "\n  {} line(s)\n  {} word(s)\n  {} character(s)\n  {} byte(s)",
                self.lines, self.words, self.chars, self.bytes
            )
        }
    }
}

impl Counter {
    fn cnt(handle: &fs::File) -> Self {
        let mut lines = 0;
        let mut chars = 0;
        let mut words = 0;
        let mut bytes = 0;

        let mut reader = BufReader::new(handle);
        let mut line = String::new();

        while let Ok(len) = reader.read_line(&mut line) {
            if len == 0 {
                break;
            }

            lines += 1;
            bytes += len;

            let mut in_word = false;
            for c in line.chars() {
                chars += 1;

                if c.is_whitespace() {
                    if in_word {
                        words += 1;
                        in_word = false;
                    }
                } else {
                    in_word = true;
                }
            }
            if in_word {
                words += 1;
            }

            line.clear();
        }

        Counter {
            lines,
            chars,
            words,
            bytes,
        }
    }
}

#[inline(always)]
fn usage() {
    println!("  Usage: {PROG_NAME} [file1] [file2] ...[file(n)]");
}
