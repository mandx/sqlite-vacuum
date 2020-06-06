use std::{
    fmt,
    fs::{metadata, File},
    io::{BufReader, Read},
    iter::Iterator,
    path::{Path, PathBuf},
};

use crate::errors::AppError;

static SQLITE_MAGIC: &[u8] = &[
    0x53, 0x51, 0x4c, 0x69, 0x74, 0x65, 0x20, 0x66, 0x6f, 0x72, 0x6d, 0x61, 0x74, 0x20, 0x33, 0x00,
];

// From https://www.sqlite.org/fileformat.html
const SQLITE_MIN_SIZE: u64 = 512;

#[derive(Debug)]
pub struct VacuumResult<'a> {
    db_file: &'a SQLiteFile,
    size_before: u64,
    size_after: u64,
}

impl<'a> VacuumResult<'a> {
    pub fn delta(&self) -> i128 {
        i128::from(self.size_before) - i128::from(self.size_after)
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

    pub fn load(path: &Path, aggresive: bool) -> Result<Option<Self>, AppError> {
        let io_error_context = AppError::io_error_wraper(path);
        let file_metadata = metadata(path).map_err(io_error_context)?;

        if !file_metadata.is_file() {
            return Ok(None);
        }

        if file_metadata.len() < SQLITE_MIN_SIZE {
            // There's a minimum file size for SQLite databases,
            // so we can skip the files that are too simply too small
            // to be considered a valid database.
            // See https://www.sqlite.org/fileformat.html for more.
            return Ok(None);
        }

        if aggresive {
            let file = File::open(path).map_err(io_error_context)?;
            for (read_byte, magic_byte) in BufReader::with_capacity(SQLITE_MAGIC.len(), file)
                .bytes()
                .zip(SQLITE_MAGIC.iter())
            {
                if read_byte.map_err(io_error_context)? != *magic_byte {
                    return Ok(None);
                }
            }

            Ok(Some(Self::new(path)))
        } else {
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("db") | Some("sqlite") => Ok(Some(Self::new(path))),
                _ => Ok(None),
            }
        }
    }

    pub fn vacuum(&self) -> Result<VacuumResult, AppError> {
        let wrap_io_error = AppError::io_error_wraper(&self.path);
        let wrap_db_open_error = AppError::db_open_error_wraper(&self.path);
        let wrap_db_exec_error = AppError::db_vacuum_error_wraper(&self.path);

        let size_before = metadata(&self.path).map_err(wrap_io_error)?.len();

        let connection = sqlite::open(&self.path).map_err(wrap_db_open_error)?;
        connection.execute("VACUUM;").map_err(wrap_db_exec_error)?;
        connection.execute("REINDEX;").map_err(wrap_db_exec_error)?;
        drop(connection);

        Ok(VacuumResult {
            db_file: &self,
            size_before,
            size_after: metadata(&self.path).map_err(wrap_io_error)?.len(),
        })
    }
}

impl fmt::Display for SQLiteFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(s) = self.path.to_str() {
            write!(f, "{}", s)
        } else {
            write!(f, "{:?}", self.path)
        }
    }
}

#[cfg(test)]
mod byte_format_tests {
    use super::*;

    #[test]
    fn test_path_accessor() {
        let path = PathBuf::from("/file");
        let db = SQLiteFile::new(&path);
        assert_eq!(db.path(), &path);
    }
}
