use std::fmt::Display;
use std::fs;
use std::io::{self, BufRead, BufReader, Error, ErrorKind, Result};
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
#[cfg(windows)]
use std::os::windows::fs::FileTypeExt;
use std::path::Path;

use clap::{ArgAction, Parser};
use colored::{Colorize, CustomColor};
use rayon::prelude::*;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "lwc", version, about, long_about = None)]
pub struct Args {
    /// One or more files or directories to process
    pub entries: Option<Vec<String>>,

    /// Recursively process directories and their contents
    #[arg(short, required = false, requires = "entries")]
    pub rflag: bool,

    /// Count special directory elements (subdirectories, FIFOs, sockets, etc.)
    /// instead of file contents
    #[arg(short, required = false, requires = "entries")]
    pub dflag: bool,

    /// Suppress per-file or per-directory stats and display only a final total
    #[arg(short, required = false, requires = "entries")]
    pub tflag: bool,

    /// Disable colors
    #[arg(short, required = false, default_value = "true", action = ArgAction::SetFalse)]
    pub cflag: bool,
}

pub struct Color;

#[derive(Debug)]
pub struct File<P: AsRef<Path>> {
    path: P,
    stat: FileStat,
}

#[derive(Debug)]
pub struct Dir<P: AsRef<Path>> {
    path: P,
    stat: DirStat,
}

#[derive(Debug)]
pub struct Stdin {
    stat: FileStat,
}

#[derive(Debug, Default)]
pub struct FileStat {
    pub lines: usize,
    pub words: usize,
    pub chars: usize,
    pub bytes: usize,
}

#[derive(Debug, Default)]
pub struct DirStat {
    pub subdirs: usize,
    pub files: usize,
    pub symlinks: usize,
    pub blocks: usize,
    pub chars: usize,
    pub fifos: usize,
    pub sockets: usize,
    pub symlink_files: usize,
    pub symlink_dirs: usize,
}

#[derive(Debug, Default)]
pub struct Total {
    file: FileStat,
    dir: DirStat,
    done: usize,
}

impl Color {
    pub fn path() -> CustomColor {
        match Self::supports_truecolor() {
            false => CustomColor::new(0, 170, 170),
            true => CustomColor::new(77, 210, 255),
        }
    }

    pub fn delim() -> CustomColor {
        match Self::supports_truecolor() {
            false => CustomColor::new(0, 0, 170),
            true => CustomColor::new(152, 251, 152),
        }
    }

    pub fn num() -> CustomColor {
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

impl<P: AsRef<Path>> File<P> {
    pub fn count(path: P) -> Result<Self> {
        if !path.as_ref().metadata()?.is_file() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("`{}` is not a regular file", path.as_ref().display()),
            ));
        }

        let f = fs::File::open(&path)?;
        let mut stat = FileStat::default();
        let mut reader = BufReader::new(f);
        let mut line = String::new();

        while let Ok(len) = reader.read_line(&mut line) {
            if len == 0 {
                break;
            }

            stat.lines += 1;
            stat.bytes += len;
            stat.chars += line.chars().count();
            stat.words += line.split_whitespace().count();

            line.clear();
        }

        Ok(Self { path, stat })
    }

    #[allow(dead_code)]
    pub fn stat(&self) -> &FileStat {
        &self.stat
    }
}

impl<P: AsRef<Path>> Dir<P> {
    pub fn count(path: P) -> Result<Self> {
        if !path.as_ref().metadata()?.is_dir() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("`{}` is not a directory", path.as_ref().display()),
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

        Ok(Self { path, stat })
    }

    #[allow(dead_code)]
    pub fn stat(&self) -> &DirStat {
        &self.stat
    }
}

impl Stdin {
    pub fn count() -> Self {
        let mut stat = FileStat::default();
        let mut reader = BufReader::new(io::stdin());
        let mut line = String::new();

        while let Ok(len) = reader.read_line(&mut line) {
            if len == 0 {
                break;
            }

            stat.lines += 1;
            stat.bytes += len;
            stat.chars += line.chars().count();
            stat.words += line.split_whitespace().count();

            line.clear();
        }

        Self { stat }
    }

    #[allow(dead_code)]
    pub fn stat(&self) -> &FileStat {
        &self.stat
    }
}

impl Total {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn file(&self) -> &FileStat {
        &self.file
    }

    pub fn dir(&self) -> &DirStat {
        &self.dir
    }

    pub fn done(&self) -> usize {
        self.done
    }

    pub fn update_file(&mut self, other: &File<impl AsRef<Path>>) {
        self.file.lines += other.stat.lines;
        self.file.words += other.stat.words;
        self.file.chars += other.stat.chars;
        self.file.bytes += other.stat.bytes;

        self.done += 1;
    }

    pub fn update_dir(&mut self, other: &Dir<impl AsRef<Path>>) {
        self.dir.subdirs += other.stat.subdirs;
        self.dir.files += other.stat.files;
        self.dir.symlinks += other.stat.symlinks;
        self.dir.blocks += other.stat.blocks;
        self.dir.chars += other.stat.chars;
        self.dir.fifos += other.stat.fifos;
        self.dir.sockets += other.stat.sockets;

        self.done += 1;
    }
}

impl<P: AsRef<Path>> Display for File<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = &self.stat;
        let path = self
            .path
            .as_ref()
            .display()
            .to_string()
            .custom_color(Color::path())
            .bold();
        let delim = "==>".custom_color(Color::delim());
        write!(f, "{stats} {delim} {path}")
    }
}

impl<P: AsRef<Path>> Display for Dir<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = &self.stat;
        let path = self
            .path
            .as_ref()
            .display()
            .to_string()
            .custom_color(Color::path())
            .bold();
        let delim = "==>".custom_color(Color::delim());
        write!(f, "{stats} {delim} {path}")
    }
}

impl Display for Stdin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.stat.fmt(f)
    }
}

impl Display for FileStat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = [
            (self.lines, "line"),
            (self.words, "word"),
            (self.chars, "char"),
            (self.bytes, "byte"),
        ];
        write!(f, "{}", format_stats(&data))
    }
}

impl Display for DirStat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = [
            (self.subdirs, "subdir"),
            (self.files, "file"),
            (self.symlinks, "symlink"),
            (self.blocks, "block"),
            (self.chars, "char"),
            (self.fifos, "fifo"),
            (self.sockets, "socket"),
            (self.symlink_files, "symlink files"),
            (self.symlink_dirs, "symlink dirs"),
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

pub enum Counter {
    File,
    Dir,
    Stdin,
}

impl Counter {
    pub fn count(&self, args: Args) -> Result<()> {
        let entries = args.entries;
        let recursive = args.rflag;
        let quiet = args.tflag;

        colored::control::set_override(args.cflag);

        match self {
            Self::File => self.count_file(
                entries
                    .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Entries not specified"))?,
                recursive,
                quiet,
            ),
            Self::Dir => self.count_dir(
                entries
                    .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Entries not specified"))?,
                recursive,
                quiet,
            ),
            Self::Stdin => self.count_stdin(),
        }
    }

    fn count_file(
        &self,
        entries: Vec<impl AsRef<Path>>,
        recursive: bool,
        quiet: bool,
    ) -> Result<()> {
        let mut total = Total::new();

        match recursive {
            false => {
                let stats: Vec<_> = entries
                    .into_iter()
                    .map(|e| e.as_ref().to_path_buf())
                    .collect();
                let stats: Vec<_> = stats.par_iter().map(File::count).collect();

                for stat in stats {
                    match stat {
                        Ok(s) => {
                            total.update_file(&s);
                            if !quiet {
                                println!("{s}");
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::InvalidInput => {
                            eprintln!("{} {e}", "==>".yellow().bold())
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            true => {
                for entry in entries {
                    let entries: Vec<_> = WalkDir::new(entry)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_file())
                        .collect();
                    let stats: Vec<_> = entries.par_iter().map(|e| File::count(e.path())).collect();

                    for stat in stats {
                        match stat {
                            Ok(s) => {
                                total.update_file(&s);
                                if !quiet {
                                    println!("{s}");
                                }
                            }
                            Err(e) if e.kind() == ErrorKind::InvalidInput => {
                                eprintln!("{} {e}", "==>".yellow().bold())
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        }

        if total.done() > 1 || (quiet && total.done() > 1) {
            println!("{}{}", if quiet { "" } else { "\n" }, total.file());
        }

        Ok(())
    }

    fn count_dir(
        &self,
        entries: Vec<impl AsRef<Path>>,
        recursive: bool,
        quiet: bool,
    ) -> Result<()> {
        let mut total = Total::new();

        match recursive {
            false => {
                let stats: Vec<_> = entries
                    .into_iter()
                    .map(|e| e.as_ref().to_path_buf())
                    .collect();
                let stats: Vec<_> = stats.par_iter().map(Dir::count).collect();

                for stat in stats {
                    match stat {
                        Ok(s) => {
                            total.update_dir(&s);
                            if !quiet {
                                println!("{s}");
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::InvalidInput => {
                            eprintln!("{} {e}", "==>".yellow().bold())
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            true => {
                for entry in entries {
                    let entries: Vec<_> = WalkDir::new(entry)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .collect();
                    let stats: Vec<_> = entries.par_iter().map(|e| Dir::count(e.path())).collect();

                    for stat in stats {
                        match stat {
                            Ok(s) => {
                                total.update_dir(&s);
                                if !quiet {
                                    println!("{s}");
                                }
                            }
                            Err(e) if e.kind() == ErrorKind::InvalidInput => {
                                eprintln!("{} {e}", "==>".yellow().bold())
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        }

        if total.done() > 1 || (quiet && total.done() > 1) {
            println!("{}{}", if quiet { "" } else { "\n" }, total.dir());
        }

        Ok(())
    }

    fn count_stdin(&self) -> Result<()> {
        let stat = Stdin::count();
        println!("{stat}");
        Ok(())
    }
}
