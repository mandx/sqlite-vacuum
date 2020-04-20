use sqlite::Error as SqliteError;
use std::{
    io::Error as IoError,
    path::{Path, PathBuf},
};
use thiserror::Error as BaseError;

#[derive(BaseError, Debug)]
pub enum AppError {
    #[error("Not a directory `{directory}`")]
    NotDirectory {
        argument: String,
        directory: PathBuf,
    },

    #[error("Error accessing directory `{directory}`: {source}")]
    DirectoryAccess { source: IoError, directory: PathBuf },

    #[error("Error accessing file `{filename}`: {source}")]
    FileAccess { source: IoError, filename: PathBuf },

    #[error("Error opening `{filename}`: {source}")]
    DatabaseOpen {
        source: SqliteError,
        filename: PathBuf,
    },

    #[error("Error vacuuming `{filename}`: {source}")]
    Vacuum {
        source: SqliteError,
        filename: PathBuf,
    },
}

impl AppError {
    pub fn not_directory<S: AsRef<str>, P: AsRef<Path>>(argument: S, directory: P) -> Self {
        AppError::NotDirectory {
            argument: argument.as_ref().into(),
            directory: directory.as_ref().into(),
        }
    }

    pub fn directory_access<P: AsRef<Path>>(source: IoError, directory: P) -> Self {
        AppError::DirectoryAccess {
            source,
            directory: directory.as_ref().into(),
        }
    }

    pub fn io_error<P: AsRef<Path>>(source: IoError, filename: P) -> Self {
        AppError::FileAccess {
            source,
            filename: filename.as_ref().into(),
        }
    }

    pub fn io_error_wraper<'a, P: Copy + AsRef<Path> + 'a>(
        filename: P,
    ) -> impl Fn(IoError) -> Self + 'a + Copy {
        move |source| Self::io_error(source, filename.as_ref())
    }

    pub fn db_open_error<P: AsRef<Path>>(source: SqliteError, filename: P) -> Self {
        AppError::DatabaseOpen {
            source,
            filename: filename.as_ref().into(),
        }
    }

    pub fn db_open_error_wraper<'a, P: Copy + AsRef<Path> + 'a>(
        filename: P,
    ) -> impl Fn(SqliteError) -> Self + 'a + Copy {
        move |source| Self::db_open_error(source, filename.as_ref())
    }

    pub fn db_vacuum_error<P: AsRef<Path>>(source: SqliteError, filename: P) -> Self {
        AppError::Vacuum {
            source,
            filename: filename.as_ref().into(),
        }
    }

    pub fn db_vacuum_error_wraper<'a, P: Copy + AsRef<Path> + 'a>(
        filename: P,
    ) -> impl Fn(SqliteError) -> Self + 'a + Copy {
        move |source| Self::db_vacuum_error(source, filename.as_ref())
    }
}
