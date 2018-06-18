extern crate sqlite;

use std::fs::{metadata, File};
use std::io::{self, Read};
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
pub enum LoadResult<T> {
    Ok(T),
    Err(io::Error),
    None,
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

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(path: &Path, aggresive: bool) -> LoadResult<Self> {
        if let Ok(metadata) = metadata(path) {
            if !metadata.is_file() {
                return LoadResult::None;
            }
        }

        if aggresive {
            match File::open(path) {
                Ok(file) => {
                    let magic: Vec<u8> = vec![
                        0x53, 0x51, 0x4c, 0x69, 0x74, 0x65, 0x20, 0x66, 0x6f, 0x72, 0x6d, 0x61,
                        0x74, 0x20, 0x33, 0x00,
                    ];

                    let mut buffer: Vec<u8> = Vec::with_capacity(magic.len());
                    // We loop over the `take` iterator instead of `collect`ing
                    // directly into the buffer vector because every byte read
                    // comes as a `Result`, and any error in any read means we
                    // end with an error.
                    for byte in file.bytes().take(magic.len()) {
                        if let Err(error) = byte {
                            return LoadResult::Err(error);
                        }
                        buffer.push(byte.unwrap())
                    }

                    if buffer != magic {
                        return LoadResult::None;
                    }

                    LoadResult::Ok(Self::new(path))
                }
                Err(error) => LoadResult::Err(error),
            }
        } else {
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("db") => LoadResult::Ok(Self::new(path)),
                Some("sqlite") => LoadResult::Ok(Self::new(path)),
                _ => LoadResult::None,
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

#[cfg(test)]
#[path = "./sqlite_file_tests.rs"]
mod byte_format_tests;
