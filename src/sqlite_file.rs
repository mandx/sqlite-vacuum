extern crate sqlite;

use std::fs::{metadata, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use failure::{Error, ResultExt};

lazy_static! {
    static ref SQLITE_MAGIC: Vec<u8> =
        b"\x53\x51\x4c\x69\x74\x65\x20\x66\x6f\x72\x6d\x61\x74\x20\x33\x00".to_vec();
}

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

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(path: &Path, aggresive: bool) -> Option<Result<Self, Error>> {
        if let Ok(metadata) = metadata(path) {
            if !metadata.is_file() {
                return None;
            }
        }

        if aggresive {
            match File::open(path).context(format!("Error checking {:?}", path)) {
                Ok(file) => {
                    let mut buffer: Vec<u8> = Vec::with_capacity(SQLITE_MAGIC.len());
                    let reader = BufReader::new(file);

                    // We loop over the `take` iterator instead of `collect`ing
                    // directly into the buffer vector because every byte read
                    // comes as a `Result`, and any error in any read means we
                    // end with an error.

                    for byte in reader.bytes().take(SQLITE_MAGIC.len()) {
                        match byte {
                            Ok(byte) => buffer.push(byte),
                            Err(error) => return Some(Err(error.into())),
                        }
                    }

                    if buffer != *SQLITE_MAGIC {
                        return None;
                    }

                    Some(Ok(Self::new(path)))
                }
                Err(error) => Some(Err(error.into())),
            }
        } else {
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("db") | Some("sqlite") => Some(Ok(Self::new(path))),
                _ => None,
            }
        }
    }

    pub fn vacuum(&self) -> Result<VacuumResult, Error> {
        let size_before = metadata(&self.path)?.len();

        sqlite::open(&self.path)
            .and_then(|connection| connection.execute("VACUUM;").and_then(|_| Ok(connection)))
            .and_then(|connection| connection.execute("REINDEX;"))
            .context(format!("Error vacuuming {:?}", self.path))
            .or_else(|error| Err(error.into()))
            .and_then(|_| {
                Ok(VacuumResult {
                    db_file: &self,
                    size_before,
                    size_after: metadata(&self.path)?.len(),
                })
            })
    }
}

#[cfg(test)]
#[path = "./sqlite_file_tests.rs"]
mod byte_format_tests;
