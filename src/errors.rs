use std::fmt;

use failure::Backtrace;
use failure::{Context, Error, Fail};

#[derive(Debug)]
struct AppError {
    inner: Context<AppErrorKind>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
enum AppErrorKind {
    #[fail(display = "Arguments error")]
    Arguments,

    #[fail(display = "File access error")]
    FileAccess,

    #[fail(display = "Database load error")]
    DatabaseLoad,

    #[fail(display = "Vacuum error")]
    Vacuum,
}

impl Fail for AppError {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl AppError {
    pub fn kind(&self) -> AppErrorKind {
        *self.inner.get_context()
    }
}

impl From<AppErrorKind> for AppError {
    fn from(kind: AppErrorKind) -> AppError {
        AppError {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<AppErrorKind>> for AppError {
    fn from(inner: Context<AppErrorKind>) -> AppError {
        AppError { inner }
    }
}
