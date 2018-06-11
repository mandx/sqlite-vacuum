extern crate clap;
extern crate sqlite;
extern crate walkdir;

use std::fs::{metadata, File};
use std::io::{self, Read};
use std::iter::Iterator;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct VacuumResult<'a> {
    db_file: &'a SQLiteFile,
    size_before: u64,
    size_after: u64,
}

impl<'a> VacuumResult<'a> {
    pub fn delta(&self) -> u64 {
        self.size_before - self.size_after
    }
}

#[derive(Debug)]
pub struct SQLiteFile {
    path: PathBuf,
}

impl SQLiteFile {
    fn new(path: &Path) -> Self {
        Self {
            path: PathBuf::from(path),
        }
    }

    pub fn path<'a>(&'a self) -> &'a Path {
        &self.path
    }

    pub fn get(path: &Path, aggresive: bool) -> Option<Self> {
        if aggresive {
            if let Ok(file) = File::open(path) {
                let magic: Vec<u8> = vec![
                    0x53, 0x51, 0x4c, 0x69, 0x74, 0x65, 0x20, 0x66, 0x6f, 0x72, 0x6d, 0x61, 0x74,
                    0x20, 0x33, 0x00,
                ];

                let buffer: Vec<u8> = file
                    .bytes()
                    .take(magic.len())
                    .map(|r| r.unwrap_or(0)) // or deal explicitly with failure!
                    .collect();

                if buffer != magic {
                    return None;
                }

                return Some(Self::new(path));
            }

            None
        } else {
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("db") => Some(Self::new(path)),
                Some("sqlite") => Some(Self::new(path)),
                _ => None,
            }
        }
    }

    pub fn vacuum<'a>(&'a self) -> io::Result<VacuumResult<'a>> {
        let size_before = metadata(&self.path)?.len();

        sqlite::open(&self.path)
            .and_then(|connection| connection.execute("VACUUM;").and_then(|_| Ok(connection)))
            .or_else(|error| Err(io::Error::new(io::ErrorKind::Other, Box::new(error))))
            .and_then(|connection| {
                connection
                    .execute("REINDEX;")
                    .or_else(|error| Err(io::Error::new(io::ErrorKind::Other, Box::new(error))))
            })
            .or_else(|error| Err(io::Error::new(io::ErrorKind::Other, Box::new(error))))
            .and_then(|_| {
                Ok(VacuumResult {
                    db_file: &self,
                    size_before,
                    size_after: metadata(&self.path)?.len(),
                })
            })
            .or_else(|error| Err(io::Error::new(io::ErrorKind::Other, Box::new(error))))
    }
}
