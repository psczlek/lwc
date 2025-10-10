use std::fmt;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::ops;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

#[cfg(windows)]
use std::os::windows::fs::FileTypeExt;

use clap::{ArgAction, Parser};
use colored::{Colorize, CustomColor};
use rayon::prelude::*;
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(name = "lwc", version, about, long_about = None)]
pub struct Args {
    /// One or more files or directories to process
    pub entries: Option<Vec<String>>,

    /// Recursively process directories and their contents
    #[arg(short = 'r', required = false, requires = "entries")]
    pub recursive: bool,

    /// Count special directory elements (subdirectories, FIFOs, sockets, etc.)
    /// instead of file contents
    #[arg(short = 'd', required = false, requires = "entries")]
    pub count_dir: bool,

    /// Suppress per-file or per-directory stats and display only a final total
    #[arg(short = 't', required = false, requires = "entries")]
    pub quiet: bool,

    /// Disable colors
    #[arg(short = 'c', required = false, default_value = "true", action = ArgAction::SetFalse)]
    pub colors: bool,
}

pub fn count(args: Args) -> io::Result<()> {
    let total = if args.count_dir {
        Arc::new(Mutex::new(Total::dir()))
    } else {
        Arc::new(Mutex::new(Total::file()))
    };

    let done = Arc::new(AtomicUsize::new(0));

    match &args.entries {
        Some(entries) if args.recursive => {
            for entry in entries {
                let entries = WalkDir::new(entry)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|e| {
                        if args.count_dir {
                            e.path().is_dir()
                        } else {
                            e.path().is_file()
                        }
                    })
                    .collect::<Vec<_>>();

                entries
                    .par_iter()
                    .map(|e| {
                        let e = e.path();
                        if args.count_dir {
                            Ok::<Stat<_>, io::Error>(Stat::Dir(dir(e)?, e))
                        } else {
                            Ok(Stat::File(file(e)?, e))
                        }
                    })
                    .for_each(|s| {
                        let total = Arc::clone(&total);
                        let done = Arc::clone(&done);
                        report_stat(s, total, done, args.quiet)
                    });
            }
        }
        Some(entries) => {
            entries
                .par_iter()
                .map(|e| {
                    if args.count_dir {
                        Ok::<Stat<_>, io::Error>(Stat::Dir(dir(e)?, e))
                    } else {
                        Ok(Stat::File(file(e)?, e))
                    }
                })
                .for_each(|s| {
                    let total = Arc::clone(&total);
                    let done = Arc::clone(&done);
                    report_stat(s, total, done, args.quiet)
                });
        }
        None => {
            let stat = stdin();
            println!("{stat}");
        }
    }

    if args.entries.is_some()
        && (done.load(Ordering::Relaxed) > 1 || (args.quiet && done.load(Ordering::Relaxed) > 1))
    {
        match total.lock() {
            Ok(total) => println!("{}{}", if args.quiet { "" } else { "\n" }, total),
            Err(e) => eprintln!("{}: {e}", "lwc".red()),
        }
    }

    Ok(())
}

fn report_stat(
    stat: io::Result<Stat<impl AsRef<Path>>>,
    total: Arc<Mutex<Total>>,
    done: Arc<AtomicUsize>,
    quiet: bool,
) {
    match stat {
        Ok(s) => match total.lock() {
            Ok(mut total) => {
                if !quiet {
                    println!("{s}");
                }
                *total += s;
                done.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => eprintln!("{}: {e}", "lwc".red()),
        },
        Err(e) => eprintln!("{}: {e}", "lwc".red()),
    }
}

enum Stat<P: AsRef<Path>> {
    File(FileStat, P),
    Dir(DirStat, P),
}

impl<P: AsRef<Path>> fmt::Display for Stat<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::File(s, p) => {
                write!(
                    f,
                    "{s} {} {}",
                    "==>".custom_color(Color::delim()),
                    p.as_ref().display().to_string().custom_color(Color::path())
                )
            }
            Self::Dir(s, p) => {
                write!(
                    f,
                    "{s} {} {}",
                    "==>".custom_color(Color::delim()),
                    p.as_ref().display().to_string().custom_color(Color::path())
                )
            }
        }
    }
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

impl<P: AsRef<Path>> ops::AddAssign<Stat<P>> for Total {
    fn add_assign(&mut self, rhs: Stat<P>) {
        match rhs {
            Stat::File(s, _) => self.update_file(&s),
            Stat::Dir(s, _) => self.update_dir(&s),
        }
    }
}

impl<P: AsRef<Path>> ops::AddAssign<&Stat<P>> for Total {
    fn add_assign(&mut self, rhs: &Stat<P>) {
        match rhs {
            Stat::File(s, _) => self.update_file(s),
            Stat::Dir(s, _) => self.update_dir(s),
        }
    }
}

impl fmt::Display for Total {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(s) => s.fmt(f),
            Self::Dir(s) => s.fmt(f),
        }
    }
}

pub fn file(path: impl AsRef<Path>) -> io::Result<FileStat> {
    if !path.as_ref().metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a regular file", path.as_ref().display()),
        ));
    }

    let f = fs::File::open(&path)?;
    let reader = BufReader::new(f);

    Ok(read_lines(reader))
}

pub fn stdin() -> FileStat {
    let reader = BufReader::new(io::stdin());
    read_lines(reader)
}

fn read_lines(mut reader: impl BufRead) -> FileStat {
    let mut stat = FileStat::default();
    let mut buf = String::new();

    while let Ok(len) = reader.read_line(&mut buf) {
        if len == 0 {
            break;
        }

        stat.lines += 1;
        stat.bytes += len;
        stat.chars += buf.chars().count();
        stat.words += buf.split_whitespace().count();

        buf.clear();
    }

    stat
}

#[derive(Debug, Default)]
pub struct FileStat {
    pub lines: usize,
    pub words: usize,
    pub chars: usize,
    pub bytes: usize,
}

impl fmt::Display for FileStat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = [
            (self.lines, "line"),
            (self.words, "word"),
            (self.chars, "char"),
            (self.bytes, "byte"),
        ];
        write!(f, "{}", format_stats(&data))
    }
}

pub fn dir(path: impl AsRef<Path>) -> io::Result<DirStat> {
    if !path.as_ref().metadata()?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a directory", path.as_ref().display()),
        ));
    }

    let entries = fs::read_dir(&path)?;
    let mut stat = DirStat::default();

    for entry in entries.flatten() {
        let metadata = entry.path().symlink_metadata()?;

        match metadata.file_type() {
            ft if ft.is_dir() => stat.subdirs += 1,
            ft if ft.is_file() => stat.files += 1,
            ft if ft.is_symlink() => stat.symlinks += 1,
            #[cfg(unix)]
            ft if ft.is_block_device() => stat.blocks += 1,
            #[cfg(unix)]
            ft if ft.is_char_device() => stat.chars += 1,
            #[cfg(unix)]
            ft if ft.is_fifo() => stat.fifos += 1,
            #[cfg(unix)]
            ft if ft.is_socket() => stat.sockets += 1,
            #[cfg(windows)]
            ft if ft.is_symlink_file() => stat.symlink_files += 1,
            #[cfg(windows)]
            ft if ft.is_symlink_dir() => stat.symlink_dirs += 1,
            _ => {}
        }
    }

    Ok(stat)
}

#[derive(Debug, Default)]
pub struct DirStat {
    pub subdirs: usize,
    pub files: usize,
    pub symlinks: usize,
    #[cfg(unix)]
    pub blocks: usize,
    #[cfg(unix)]
    pub chars: usize,
    #[cfg(unix)]
    pub fifos: usize,
    #[cfg(unix)]
    pub sockets: usize,
    #[cfg(windows)]
    pub symlink_files: usize,
    #[cfg(windows)]
    pub symlink_dirs: usize,
}

impl fmt::Display for DirStat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = [
            (self.subdirs, "subdir"),
            (self.files, "file"),
            (self.symlinks, "symlink"),
            #[cfg(unix)]
            (self.blocks, "block"),
            #[cfg(unix)]
            (self.chars, "char"),
            #[cfg(unix)]
            (self.fifos, "fifo"),
            #[cfg(unix)]
            (self.sockets, "socket"),
            #[cfg(windows)]
            (self.symlink_files, "symlink file"),
            #[cfg(windows)]
            (self.symlink_dirs, "symlink dir"),
        ];
        write!(f, "{}", format_stats(&data))
    }
}

fn format_stats(stats: &[(usize, &str)]) -> String {
    stats
        .iter()
        .filter(|(count, _)| *count >= 1)
        .map(|(count, what)| {
            let what = if *count > 1 {
                format!("{what}s")
            } else {
                what.to_string()
            };
            let count = count.to_string().custom_color(Color::num());

            format!("{count} {what}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

struct Color;

impl Color {
    fn path() -> CustomColor {
        match Self::supports_truecolor() {
            false => CustomColor::new(0, 170, 170),
            true => CustomColor::new(77, 210, 255),
        }
    }

    fn delim() -> CustomColor {
        match Self::supports_truecolor() {
            false => CustomColor::new(0, 0, 170),
            true => CustomColor::new(152, 251, 152),
        }
    }

    fn num() -> CustomColor {
        match Self::supports_truecolor() {
            false => CustomColor::new(170, 170, 0),
            true => CustomColor::new(255, 218, 185),
        }
    }

    fn supports_truecolor() -> bool {
        std::env::var("COLORTERM")
            .map(|colorterm| colorterm == "truecolor" || colorterm == "24bit")
            .unwrap_or(false)
    }
}
