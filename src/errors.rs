// use std::io;
use std::path::PathBuf;

error_chain! {
    errors {
        ArgumentsError(t: String) {
            description("Arguments error")
            display("{}", t)
        }

        FileAccessError(p: PathBuf) {
            description("File access error")
            display("Error accessing file {:?}", p)
        }

        DatabaseLoadError(p: PathBuf) {
            description("Database load error")
            display("Error loading database {:?}", p)
        }

        VacuumError(p: PathBuf) {
            description("Database vacuum error")
            display("Error vacuuming database {:?}", p)
        }
    }
}
