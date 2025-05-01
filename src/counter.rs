use resolve_path::PathResolveExt;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::FileTypeExt;

pub struct Counter;

pub struct FileStat {
    pub path: String,
    pub lines: usize,
    pub chars: usize,
    pub words: usize,
    pub bytes: usize,
    pub is_binary: bool,
}

pub struct DirStat {
    pub path: String,
    pub subdirs: usize,
    pub files: usize,
    pub symlinks: usize,
    pub blocks: usize,
    pub chars: usize,
    pub fifos: usize,
    pub sockets: usize,
}

impl Counter {
    pub fn new() -> Self {
        Counter
    }

    pub fn fcount(&self, path: &str) -> Option<FileStat> {
        let file = File::open(path.resolve()).ok()?;
        let metadata = file.metadata().ok()?;
        if !metadata.file_type().is_file() {
            return None;
        }

        let mut stat = FileStat {
            path: path.to_string(),
            lines: 0,
            chars: 0,
            words: 0,
            bytes: 0,
            is_binary: false,
        };

        let mut reader = BufReader::new(file);
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

        Some(stat)
    }

    pub fn fcounts(&self, paths: &Vec<String>) -> Vec<Option<FileStat>> {
        let mut stats: Vec<Option<FileStat>> = Vec::new();
        for file in paths {
            let stat = self.fcount(file);
            stats.push(stat);
        }
        stats
    }

    pub fn dcount(&self, path: &str) -> Option<DirStat> {
        if !path.resolve().metadata().ok()?.is_dir() {
            return None;
        }
        let mut stat = DirStat {
            path: path.to_string(),
            subdirs: 0,
            files: 0,
            symlinks: 0,
            blocks: 0,
            chars: 0,
            fifos: 0,
            sockets: 0,
        };

        if let Ok(dir_entries) = fs::read_dir(path.resolve()) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                let metadata = path.metadata().ok()?;
                match metadata.file_type() {
                    ft if ft.is_dir() => stat.subdirs += 1,
                    ft if ft.is_file() => stat.files += 1,
                    ft if ft.is_symlink() => stat.symlinks += 1,
                    ft if ft.is_block_device() => stat.blocks += 1,
                    ft if ft.is_char_device() => stat.chars += 1,
                    ft if ft.is_fifo() => stat.fifos += 1,
                    ft if ft.is_socket() => stat.sockets += 1,
                    _ => {}
                }
            }
        }

        Some(stat)
    }

    pub fn dcounts(&self, paths: &Vec<String>) -> Vec<Option<DirStat>> {
        let mut stats: Vec<Option<DirStat>> = Vec::new();
        for entry in paths {
            let stat = self.dcount(entry);
            stats.push(stat);
        }
        stats
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}
