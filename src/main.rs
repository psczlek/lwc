use clap::Parser;
use colored::*;
use lwc::counter::{Counter, DirStat, FileStat};
use lwc::flag::Flag;
use resolve_path::PathResolveExt;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let flags = Flag::parse();
    let counter = Counter::new();
    let mut entries = flags.entries;

    if !flags.fflag {
        if flags.rflag {
            let mut collected_entries = Vec::new();

            for entry in &entries {
                let p = PathBuf::from(entry);
                if p.is_dir() {
                    collect_files(&p, &mut collected_entries);
                } else {
                    collected_entries.push(entry.clone());
                }
            }

            entries = collected_entries;
        }

        let mut total = TotalFileStat::default();
        let stats = counter.fcounts(&entries);
        for stat in stats.into_iter().flatten() {
            file_stat_print(&stat, flags.oflag, flags.aflag);
            total.lines += stat.lines;
            total.words += stat.words;
            total.chars += stat.chars;
            total.bytes += stat.bytes;
        }

        println!("{total}");
    } else {
        if flags.rflag {
            let mut collected_entries = Vec::new();

            for entry in &entries {
                let p = PathBuf::from(entry);
                if p.is_dir() {
                    collect_entries(&p, &mut collected_entries);
                } else {
                    collected_entries.push(entry.clone());
                }
            }

            entries = collected_entries;
        }

        let mut total = TotalDirStat::default();
        let stats = counter.dcounts(&entries);
        for stat in stats.into_iter().flatten() {
            dir_stat_print(&stat, flags.oflag, flags.aflag);
            total.subdirs += stat.subdirs;
            total.files += stat.files;
            total.symlinks += stat.symlinks;
            total.blocks += stat.blocks;
            total.chars += stat.chars;
            total.fifos += stat.fifos;
            total.sockets += stat.sockets;
        }

        println!("{total}");
    }

    ExitCode::SUCCESS
}

#[derive(Default)]
struct TotalFileStat {
    lines: usize,
    chars: usize,
    words: usize,
    bytes: usize,
}

impl Display for TotalFileStat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n{} = {} lines, {} words, {} characters, {} bytes",
            "Total".custom_color(CustomColor::new(36, 244, 123)),
            self.lines
                .to_string()
                .custom_color(CustomColor::new(241, 163, 111)),
            self.words
                .to_string()
                .custom_color(CustomColor::new(241, 163, 111)),
            self.chars
                .to_string()
                .custom_color(CustomColor::new(241, 163, 111)),
            self.bytes
                .to_string()
                .custom_color(CustomColor::new(241, 163, 111)),
        )
    }
}

#[derive(Default)]
struct TotalDirStat {
    subdirs: usize,
    files: usize,
    symlinks: usize,
    blocks: usize,
    chars: usize,
    fifos: usize,
    sockets: usize,
}

impl Display for TotalDirStat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n{} = {} subdirs, {} files, {} symlinks, {} blocks {} chars {} fifos {} sockets",
            "Total".custom_color(CustomColor::new(36, 244, 123)),
            self.subdirs
                .to_string()
                .custom_color(CustomColor::new(241, 163, 111)),
            self.files
                .to_string()
                .custom_color(CustomColor::new(241, 163, 111)),
            self.symlinks
                .to_string()
                .custom_color(CustomColor::new(241, 163, 111)),
            self.blocks
                .to_string()
                .custom_color(CustomColor::new(241, 163, 111)),
            self.chars
                .to_string()
                .custom_color(CustomColor::new(241, 163, 111)),
            self.fifos
                .to_string()
                .custom_color(CustomColor::new(241, 163, 111)),
            self.sockets
                .to_string()
                .custom_color(CustomColor::new(241, 163, 111)),
        )
    }
}

fn collect_files(dir: &Path, files: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files(&path, files);
            } else if let Some(path_str) = path.to_str() {
                files.push(path_str.to_string());
            }
        }
    }
}

fn collect_entries(dir: &Path, entries: &mut Vec<String>) {
    if let Ok(dir_entries) = fs::read_dir(dir) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            if let Some(path_str) = path.to_str() {
                entries.push(path_str.to_string());
            }
            if path.is_dir() {
                collect_entries(&path, entries);
            }
        }
    }
}

fn file_stat_print(stat: &FileStat, oneline: bool, absolute_paths: bool) {
    if stat.lines == 0 && stat.words == 0 && stat.chars == 0 && stat.bytes == 0 {
        println!(
            "{}: -",
            if !absolute_paths {
                stat.path.custom_color(CustomColor::new(42, 195, 222))
            } else {
                stat.path
                    .resolve()
                    .to_str()
                    .unwrap_or("Failed to resolve path")
                    .custom_color(CustomColor::new(42, 195, 222))
            },
        );
    } else {
        let cnts = [
            (stat.lines, "line"),
            (stat.words, "word"),
            (stat.chars, "character"),
            (stat.bytes, "byte"),
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
                .custom_color(CustomColor::new(241, 163, 111));
            let mut label = String::from(label);

            if num == 0 || num > 1 {
                label.push('s');
            }

            if !oneline {
                format!(
                    "{num_str:>padding_left$} {label:>padding_rigth$}\n",
                    padding_left = (num_str.len() + 2),
                    padding_rigth = (max_num_len - num_str.len() + label.len()),
                )
            } else {
                format!("{num_str} {label} ")
            }
        }));

        println!(
            "{}:{}{}",
            if !absolute_paths {
                stat.path.custom_color(CustomColor::new(42, 195, 222))
            } else {
                stat.path
                    .resolve()
                    .to_str()
                    .unwrap_or("Failed to resolve path")
                    .custom_color(CustomColor::new(42, 195, 222))
            },
            if !oneline { "\n" } else { " " },
            output
        );
    }
}

fn dir_stat_print(stat: &DirStat, oneline: bool, absolute_paths: bool) {
    if stat.subdirs == 0
        && stat.files == 0
        && stat.symlinks == 0
        && stat.blocks == 0
        && stat.chars == 0
        && stat.fifos == 0
        && stat.sockets == 0
    {
        println!(
            "{}: -",
            if !absolute_paths {
                stat.path.custom_color(CustomColor::new(42, 195, 222))
            } else {
                stat.path
                    .resolve()
                    .to_str()
                    .unwrap_or("Failed to resolve path")
                    .custom_color(CustomColor::new(42, 195, 222))
            },
        );
    } else {
        let cnts = [
            (stat.subdirs, "subdir"),
            (stat.files, "file"),
            (stat.symlinks, "symlink"),
            (stat.blocks, "block"),
            (stat.chars, "char"),
            (stat.fifos, "fifo"),
            (stat.sockets, "socket"),
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
                .custom_color(CustomColor::new(241, 163, 111));
            let mut label = String::from(label);

            if num == 0 || num > 1 {
                label.push('s');
            }

            if !oneline {
                format!(
                    "{num_str:>padding_left$} {label:>padding_rigth$}\n",
                    padding_left = (num_str.len() + 2),
                    padding_rigth = (max_num_len - num_str.len() + label.len()),
                )
            } else {
                format!("{num_str} {label} ")
            }
        }));

        println!(
            "{}:{}{}",
            if !absolute_paths {
                stat.path.custom_color(CustomColor::new(42, 195, 222))
            } else {
                stat.path
                    .resolve()
                    .to_str()
                    .unwrap_or("Failed to resolve path")
                    .custom_color(CustomColor::new(42, 195, 222))
            },
            if !oneline { "\n" } else { " " },
            output
        );
    }
}
