extern crate clap;
extern crate sqlite;
extern crate walkdir;

use std::fmt;
use std::fs::{metadata, File};
use std::io::Read;
use std::iter::Iterator;
use std::path::{Path, PathBuf};

fn size_fmt(size: Option<u64>) -> String {
    match size {
        Some(value) => value.to_string(),
        None => "?".to_string(),
    }
}

#[derive(Debug)]
pub struct SQLiteFile {
    path: PathBuf,
    size_before: Option<u64>,
    size_after: Option<u64>,
}

impl fmt::Display for SQLiteFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} ({} -> {})",
            self.path.to_str().unwrap_or("<path?>"),
            size_fmt(self.size_before),
            size_fmt(self.size_after)
        )
    }
}

impl SQLiteFile {
    fn new(path: &Path) -> Self {
        Self {
            size_before: match metadata(path) {
                Ok(metadata) => Some(metadata.len()),
                Err(_) => None,
            },
            size_after: None,
            path: PathBuf::from(path),
        }
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

    pub fn vacuum(&mut self) -> Result<(), sqlite::Error> {
        sqlite::open(&self.path)
            .and_then(|connection| {
                println!("Connected to {:?}", &self.path);
                connection.execute("VACUUM;").and_then(|_| {
                    println!("Vacuum'd {:?}", &self.path);
                    Ok(connection)
                })
            })
            .and_then(|connection| {
                connection.execute("REINDEX;").and_then(|_| {
                    println!("Reindexed {:?}", &self.path);
                    Ok(())
                })
            })
            .and_then(|_| {
                if let Ok(metadata) = metadata(&self.path) {
                    self.size_after = Some(metadata.len());
                }
                Ok(())
            })
    }

    pub fn delta(&self) -> Option<u64> {
        match (self.size_before, self.size_after) {
            (Some(size_before), Some(size_after)) => Some(size_after - size_before),
            _ => None,
        }
    }
}
