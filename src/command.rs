use std::collections::HashMap;
use std::io;
use std::ops;
use std::path::{Path, PathBuf};
use std::thread;

use clap::{ArgAction, Parser};
use colored::Colorize;

use tabled::builder::Builder as TableBuilder;
use tabled::settings::object::{Columns, Rows};
use tabled::settings::themes::{Colorization, Theme};
use tabled::settings::{Color, Panel, Style};

use crate::counter::{self, DirStat, FileStat, Stat, Which};

#[derive(Debug, Parser)]
#[command(name = "lwc", version, about, long_about = None)]
struct Args {
    /// One or more files or directories to process.
    pub paths: Option<Vec<PathBuf>>,

    /// Recursively process directories and their contents.
    #[arg(short = 'r', required = false, requires = "paths")]
    pub recursive: bool,

    /// Count special directory elements (subdirectories, FIFOs, sockets, etc.).
    /// instead of file contents
    #[arg(short = 'd', required = false, requires = "paths")]
    pub count_dir: bool,

    /// Suppress per-file or per-directory stats and display only a final total.
    #[arg(short = 't', required = false, requires = "paths")]
    pub quiet: bool,

    /// Specify the number of threads to use.
    #[arg(short = 'T', required = false, requires = "paths")]
    pub threads: Option<usize>,

    /// Print the number of lines in each input file.
    #[arg(short = 'l', required = false)]
    pub print_lines: bool,

    /// Print the number of words in each input file.
    #[arg(short = 'w', required = false)]
    pub print_words: bool,

    /// Print the number of characters in each input file.
    #[arg(short = 'c', required = false)]
    pub print_chars: bool,

    /// Print the number of bytes in each input file.
    #[arg(short = 'b', required = false)]
    pub print_bytes: bool,

    /// Print the number of subdirectories in each input directory.
    #[arg(short = 's', required = false, requires = "count_dir")]
    pub print_subdirs: bool,

    /// Print the number of files in each input directory.
    #[arg(short = 'f', required = false, requires = "count_dir")]
    pub print_files: bool,

    /// Print the number of symbolic links in each input directory.
    #[arg(short = 'L', required = false, requires = "count_dir")]
    pub print_symlinks: bool,

    /// Print the number of block devices in each input directory.
    #[cfg(unix)]
    #[arg(short = 'B', required = false, requires = "count_dir")]
    pub print_blocks: bool,

    /// Print the number of character devices in each input directory.
    #[cfg(unix)]
    #[arg(short = 'D', required = false, requires = "count_dir")]
    pub print_chards: bool,

    /// Print the number of FIFOs in each input directory.
    #[cfg(unix)]
    #[arg(short = 'F', required = false, requires = "count_dir")]
    pub print_fifos: bool,

    /// Print the number of sockets in each input directory.
    #[cfg(unix)]
    #[arg(short = 'S', required = false, requires = "count_dir")]
    pub print_sockets: bool,

    /// Print the number of symbolic link files (a symbolic link that is also a file)
    /// in each input directory.
    #[cfg(windows)]
    #[arg(short = 'x', required = false, requires = "count_dir")]
    pub print_symlink_files: bool,

    /// Print the number of symbolic link directories (a symbolic link that is
    /// also a directory) in each input directory.
    #[cfg(windows)]
    #[arg(short = 'X', required = false, requires = "count_dir")]
    pub print_symlink_dirs: bool,

    /// Disable colors
    #[arg(short = 'C', required = false, default_value = "true", action = ArgAction::SetFalse)]
    pub colors: bool,
}

pub fn run() -> io::Result<()> {
    let args = Args::parse();

    colored::control::set_override(args.colors);

    match &args.paths {
        Some(paths) => {
            let stats = counter::count_many(
                paths,
                if args.count_dir {
                    Which::Dir
                } else {
                    Which::File
                },
                args.recursive,
                args.threads
                    .unwrap_or_else(|| match thread::available_parallelism() {
                        Ok(n) => n.get(),
                        Err(e) => {
                            eprintln!(
                                "{}: Failed to retrieve the number of CPUs: {e}",
                                "lwc".red()
                            );
                            1
                        }
                    }),
            )?;

            print_stats(&stats, &args);
        }
        None => {
            let stat = counter::stdin()?;
            print_stdin_stats(&stat, &args);
        }
    }

    Ok(())
}

#[derive(Debug)]
enum Total {
    File(FileStat),
    Dir(DirStat),
}

impl Total {
    fn file() -> Self {
        Self::File(FileStat::default())
    }

    fn dir() -> Self {
        Self::Dir(DirStat::default())
    }

    fn update_file(&mut self, fs: &FileStat) {
        match self {
            Self::File(s) => {
                s.lines += fs.lines;
                s.words += fs.words;
                s.chars += fs.chars;
                s.bytes += fs.bytes;
            }
            Self::Dir(_) => (),
        }
    }

    fn update_dir(&mut self, ds: &DirStat) {
        match self {
            Self::File(_) => (),
            Self::Dir(s) => {
                s.subdirs += ds.subdirs;
                s.files += ds.files;
                s.symlinks += ds.symlinks;

                #[cfg(unix)]
                if cfg!(unix) {
                    s.blocks += ds.blocks;
                    s.chars += ds.chars;
                    s.fifos += ds.fifos;
                    s.sockets += ds.sockets;
                }

                #[cfg(windows)]
                if cfg!(windows) {
                    s.symlink_files += ds.symlink_files;
                    s.symlink_dirs += ds.symlink_dirs;
                }
            }
        }
    }
}

impl ops::AddAssign<Stat> for Total {
    fn add_assign(&mut self, rhs: Stat) {
        match rhs {
            Stat::File(s) => self.update_file(&s),
            Stat::Dir(s) => self.update_dir(&s),
        }
    }
}

impl ops::AddAssign<&Stat> for Total {
    fn add_assign(&mut self, rhs: &Stat) {
        match rhs {
            Stat::File(s) => self.update_file(s),
            Stat::Dir(s) => self.update_dir(s),
        }
    }
}

fn print_stats(stats: &HashMap<PathBuf, io::Result<Stat>>, args: &Args) {
    let mut table_builder = TableBuilder::new();

    add_columns(&mut table_builder, args);

    let mut errors = 0;
    let mut total = if args.count_dir {
        Total::dir()
    } else {
        Total::file()
    };

    for (path, stat) in stats {
        match stat {
            Ok(s) => {
                if !args.quiet {
                    match s {
                        Stat::File(fs) => add_file_row(&mut table_builder, path, fs, args),
                        Stat::Dir(ds) => add_dir_row(&mut table_builder, path, ds, args),
                    }
                }

                total += s;
            }
            Err(e) => {
                eprintln!("{}: {e}", path.display().to_string().red());
                errors += 1;
            }
        }
    }

    if errors >= 1 {
        println!();
    }

    match &total {
        Total::File(fs) => add_file_row(&mut table_builder, "total", fs, args),
        Total::Dir(ds) => add_dir_row(&mut table_builder, "total", ds, args),
    }

    let mut table = table_builder.build();
    let mut theme = Theme::from(Style::modern_rounded());

    if errors > 0 {
        table.with(Panel::footer(format!("errors: {errors}")));
    }

    if args.colors {
        theme.set_colors_top(Color::FG_BRIGHT_BLACK);
        theme.set_colors_bottom(Color::FG_BRIGHT_BLACK);
        theme.set_colors_left(Color::FG_BRIGHT_BLACK);
        theme.set_colors_right(Color::FG_BRIGHT_BLACK);
        theme.set_colors_corner_top_left(Color::FG_BRIGHT_BLACK);
        theme.set_colors_corner_top_right(Color::FG_BRIGHT_BLACK);
        theme.set_colors_corner_bottom_left(Color::FG_BRIGHT_BLACK);
        theme.set_colors_corner_bottom_right(Color::FG_BRIGHT_BLACK);
        theme.set_colors_intersection_bottom(Color::FG_BRIGHT_BLACK);
        theme.set_colors_intersection_top(Color::FG_BRIGHT_BLACK);
        theme.set_colors_intersection_right(Color::FG_BRIGHT_BLACK);
        theme.set_colors_intersection_left(Color::FG_BRIGHT_BLACK);
        theme.set_colors_intersection(Color::FG_BRIGHT_BLACK);
        theme.set_colors_horizontal(Color::FG_BRIGHT_BLACK);
        theme.set_colors_vertical(Color::FG_BRIGHT_BLACK);

        table
            .with(Colorization::exact(
                Some(Color::FG_CYAN | Color::BOLD),
                Columns::first(),
            ))
            .with(Colorization::exact(
                Some(Color::FG_GREEN | Color::BOLD),
                Rows::first(),
            ));

        if errors > 0 {
            table.with(Colorization::exact(
                Some(Color::FG_RED | Color::BOLD),
                Rows::last(),
            ));
        }
    }

    table.with(theme);

    println!("{table}");
}

fn print_stdin_stats(fs: &FileStat, args: &Args) {
    let stats = [
        ("line", fs.lines, args.print_lines),
        ("word", fs.words, args.print_words),
        ("char", fs.chars, args.print_chars),
        ("byte", fs.bytes, args.print_bytes),
    ];

    let fmt = stats
        .iter()
        .filter(|(_, count, print)| {
            if stats.iter().all(|(_, _, print)| !*print) {
                *count >= 1
            } else {
                *print
            }
        })
        .map(|(name, count, _)| {
            let name = if *count == 0 || *count > 1 {
                format!("{name}s")
            } else {
                name.to_string()
            };

            let count = if *count > 0 {
                count.to_string().green()
            } else {
                count.to_string().yellow()
            };

            format!("{count} {name}")
        })
        .collect::<Vec<_>>()
        .join(" ");

    println!("{fmt}");
}

fn add_columns(table_builder: &mut TableBuilder, args: &Args) {
    let mut columns = vec![String::new()];

    if args.count_dir {
        let dir_columns = [
            ("subdirs", args.print_subdirs),
            ("files", args.print_files),
            ("symlinks", args.print_symlinks),
            #[cfg(unix)]
            ("blocks", args.print_blocks),
            #[cfg(unix)]
            ("chars", args.print_chards),
            #[cfg(unix)]
            ("fifos", args.print_fifos),
            #[cfg(unix)]
            ("sockets", args.print_sockets),
            #[cfg(windows)]
            ("symlink files", args.print_symlink_files),
            #[cfg(windows)]
            ("symlink dirs", args.print_symlink_dirs),
        ];

        let no_flags_set = dir_columns.iter().all(|(_, enabled)| !*enabled);

        if no_flags_set {
            dir_columns
                .iter()
                .for_each(|(name, _)| columns.push((*name).to_owned()));
        } else {
            for (name, enabled) in dir_columns {
                if enabled {
                    columns.push(name.to_owned());
                }
            }
        }
    } else {
        let file_columns = [
            ("lines", args.print_lines),
            ("words", args.print_words),
            ("chars", args.print_chars),
            ("bytes", args.print_bytes),
        ];

        let no_flags_set = file_columns.iter().all(|(_, enabled)| !*enabled);

        if no_flags_set {
            file_columns
                .iter()
                .for_each(|(name, _)| columns.push((*name).to_owned()));
        } else {
            for (name, enabled) in file_columns {
                if enabled {
                    columns.push(name.to_owned());
                }
            }
        }
    }

    table_builder.push_record(columns);
}

fn add_file_row(
    table_builder: &mut TableBuilder,
    path: impl AsRef<Path>,
    fs: &FileStat,
    args: &Args,
) {
    let mut row = vec![path.as_ref().display().to_string()];

    let stats = [
        (fs.lines, args.print_lines),
        (fs.words, args.print_words),
        (fs.chars, args.print_chars),
        (fs.bytes, args.print_bytes),
    ];

    let no_flags_set = stats.iter().all(|(_, enabled)| !*enabled);

    if no_flags_set {
        stats
            .iter()
            .for_each(|(value, _)| row.push(value.to_string()));
    } else {
        for (value, enabled) in stats {
            if enabled {
                row.push(value.to_string());
            }
        }
    }

    table_builder.push_record(row);
}

fn add_dir_row(
    table_builder: &mut TableBuilder,
    path: impl AsRef<Path>,
    ds: &DirStat,
    args: &Args,
) {
    let mut row = vec![path.as_ref().display().to_string()];

    let dir_metrics = [
        (ds.subdirs, args.print_subdirs),
        (ds.files, args.print_files),
        (ds.symlinks, args.print_symlinks),
        #[cfg(unix)]
        (ds.blocks, args.print_blocks),
        #[cfg(unix)]
        (ds.chars, args.print_chards),
        #[cfg(unix)]
        (ds.fifos, args.print_fifos),
        #[cfg(unix)]
        (ds.sockets, args.print_sockets),
        #[cfg(windows)]
        (ds.symlink_files, args.print_symlink_files),
        #[cfg(windows)]
        (ds.symlink_dirs, args.print_symlink_dirs),
    ];

    let no_flags_set = dir_metrics.iter().all(|(_, enabled)| !*enabled);

    if no_flags_set {
        dir_metrics
            .iter()
            .for_each(|(value, _)| row.push(value.to_string()));
    } else {
        for (value, enabled) in dir_metrics {
            if enabled {
                row.push(value.to_string());
            }
        }
    }

    table_builder.push_record(row);
}
