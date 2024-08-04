use colored::*;
use core::fmt;
use std::env;
use std::fs;
use std::io;
use std::io::{BufRead, BufReader};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        usage(&args[0]);
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
                    println!(
                        "{}: not a regular file! - SKIPPED\n",
                        file.custom_color(CustomColor::new(42, 195, 222)).bold()
                    );
                    continue;
                }
                f
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    println!(
                        "warning: '{}' not found!\n",
                        file.custom_color(CustomColor::new(42, 195, 222)).bold()
                    );
                    continue;
                }
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        };

        let counter = Counter::cnt(&fhandle);
        println!(
            "{}: {}",
            file.custom_color(CustomColor::new(42, 195, 222)).bold(),
            counter
        );

        total_lines += counter.lines;
        total_words += counter.words;
        total_chars += counter.chars;
        total_bytes += counter.bytes;
    }

    println!(
        "Total: {} lines, {} words, {} characters, {} bytes",
        total_lines
            .to_string()
            .custom_color(CustomColor::new(241, 163, 111))
            .bold(),
        total_words
            .to_string()
            .custom_color(CustomColor::new(241, 163, 111))
            .bold(),
        total_chars
            .to_string()
            .custom_color(CustomColor::new(241, 163, 111))
            .bold(),
        total_bytes
            .to_string()
            .custom_color(CustomColor::new(241, 163, 111))
            .bold(),
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
            write!(f, "<EMPTY>")
        } else {
            let cnts = [
                (self.lines, "line"),
                (self.words, "word"),
                (self.chars, "character"),
                (self.bytes, "byte"),
            ];

            let max_num_len = cnts
                .iter()
                .map(|&(num, _)| num.to_string().len())
                .max()
                .unwrap_or(0);

            let mut output = String::new();
            output.extend(cnts.iter().map(|&(num, label)| {
                let num_str = num
                    .to_string()
                    .custom_color(CustomColor::new(241, 163, 111))
                    .bold();
                let mut label = String::from(label);

                if num > 1 {
                    label.push('s');
                }

                format!(
                    "{num_str:>padding_left$} {label:>padding_rigth$}\n",
                    padding_left = (num_str.len() + 2),
                    padding_rigth = (max_num_len - num_str.len() + label.len()),
                )
            }));

            write!(f, "\n{output}")
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
fn usage(prog_name: &str) {
    println!("usage: {prog_name} [file1] [file2] ...[file(n)]");
}
