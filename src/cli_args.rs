extern crate clap;

use std::env::current_dir;
use std::fs::metadata;
use std::path::PathBuf;

use clap::{App, Arg};

use super::errors::*;

#[derive(Debug)]
pub struct Arguments {
    pub directory: PathBuf,
    pub aggresive: bool,
}

impl Arguments {
    pub fn get() -> Result<Self> {
        let app = App::new("sqlite-vacuum")
            .arg(
                Arg::with_name("directory")
                    .value_name("DIRECTORY")
                    .help("Sets the directory to walk")
                    .required(false),
            )
            .arg(
                Arg::with_name("aggresive")
                    .short("a")
                    .long("aggresive")
                    .help("Inspect the file's header to check if it is a SQLite database, instead of just checking the extension (which is faster, but it can lead to false positives).")
                    .takes_value(false)
                    .required(false)
            );

        let matches = app.get_matches_safe()
            .chain_err(|| ErrorKind::ArgumentsError(String::from("Invalid arguments")))?;

        let cwd = match matches.value_of("directory") {
            Some(arg_value) => {
                let path = PathBuf::from(&arg_value);

                let path_meta = metadata(&path).chain_err(|| {
                    ErrorKind::ArgumentsError(format!(
                        "`{}` is not a valid or accessible path",
                        arg_value
                    ))
                })?;

                if !path_meta.is_dir() {
                    return Err(Error::from_kind(ErrorKind::ArgumentsError(format!(
                        "`{}` is not a directory",
                        arg_value
                    ))));
                }

                path
            }
            None => current_dir().chain_err(|| {
                ErrorKind::ArgumentsError(String::from("Can not access current working dir"))
            })?,
        };

        let aggresive = matches.is_present("aggresive");

        Ok(Self {
            directory: cwd,
            aggresive,
        })
    }
}
