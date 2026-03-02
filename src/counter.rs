use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::ops;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

#[cfg(windows)]
use std::os::windows::fs::FileTypeExt;

use rayon::ThreadPoolBuilder;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct FileStat {
    pub lines: usize,
    pub words: usize,
    pub chars: usize,
    pub bytes: usize,
}

impl FileStat {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ops::AddAssign for FileStat {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self {
            lines: self.lines + rhs.lines,
            words: self.words + rhs.words,
            chars: self.chars + rhs.chars,
            bytes: self.bytes + rhs.bytes,
        }
    }
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

impl DirStat {
    pub fn new() -> Self {
        DirStat::default()
    }
}

impl ops::AddAssign for DirStat {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self {
            subdirs: self.subdirs + rhs.subdirs,
            files: self.files + rhs.files,
            symlinks: self.symlinks + rhs.symlinks,
            #[cfg(unix)]
            blocks: self.blocks + rhs.blocks,
            #[cfg(unix)]
            chars: self.chars + rhs.chars,
            #[cfg(unix)]
            fifos: self.fifos + rhs.fifos,
            #[cfg(unix)]
            sockets: self.sockets + rhs.sockets,
            #[cfg(windows)]
            symlink_files: self.symlink_files + rhs.symlink_files,
            #[cfg(windows)]
            symlink_dirs: self.symlink_dirs + rhs.symlink_dirs,
        }
    }
}

#[derive(Debug)]
pub enum Stat {
    File(FileStat),
    Dir(DirStat),
}

impl From<FileStat> for Stat {
    fn from(value: FileStat) -> Self {
        Self::File(value)
    }
}

impl From<DirStat> for Stat {
    fn from(value: DirStat) -> Self {
        Self::Dir(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Which {
    File,
    Dir,
}

pub fn count_many(
    paths: &[impl AsRef<Path>],
    which: Which,
    recursive: bool,
    threads: usize,
) -> io::Result<HashMap<PathBuf, io::Result<Stat>>> {
    assert_ne!(threads, 0);

    let workers = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|e| io::Error::other(format!("Failed to build workers pool: {e}")))?;

    let mut entries = vec![];

    for path in paths {
        if recursive {
            for entry in WalkDir::new(path) {
                let entry = entry?;
                let p = entry.path();

                match which {
                    Which::File if p.is_dir() => continue,
                    Which::Dir if p.is_file() => continue,
                    _ => {}
                }

                entries.push(p.to_path_buf());
            }
        } else {
            entries.push(path.as_ref().to_path_buf());
        }
    }

    let stats = workers.install(|| {
        entries
            .into_par_iter()
            .map(|path| {
                let stat = count(&path, which);
                Ok((path, stat))
            })
            .collect::<io::Result<HashMap<_, _>>>()
    })?;

    Ok(stats)
}

pub fn count(path: impl AsRef<Path>, which: Which) -> io::Result<Stat> {
    match which {
        Which::File => file(path).map(Stat::from),
        Which::Dir => dir(path).map(Stat::from),
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
    let reader = BufReader::with_capacity(16 * 1024, f);

    read_lines(reader)
}

pub fn dir(path: impl AsRef<Path>) -> io::Result<DirStat> {
    if !path.as_ref().metadata()?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a directory", path.as_ref().display()),
        ));
    }

    let entries = fs::read_dir(&path)?;
    let mut stat = DirStat::new();

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

pub fn stdin() -> io::Result<FileStat> {
    let reader = BufReader::new(io::stdin());
    read_lines(reader)
}

fn read_lines(mut reader: impl BufRead) -> io::Result<FileStat> {
    let mut stat = FileStat::new();
    let mut buf = String::new();

    loop {
        let len = reader.read_line(&mut buf)?;
        if len == 0 {
            break;
        }

        stat.lines += 1;
        stat.bytes += len;
        stat.chars += buf.chars().count();
        stat.words += buf.split_whitespace().count();

        buf.clear();
    }

    Ok(stat)
}
