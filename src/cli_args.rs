extern crate clap;

use std::env::current_dir;
use std::fs::metadata;
use std::path::PathBuf;

use clap::{App, Arg};

#[derive(Debug)]
pub struct Arguments {
    pub directory: PathBuf,
    pub aggresive: bool,
}

impl Arguments {
    pub fn get() -> Result<Self, clap::Error> {
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

        let matches = match app.get_matches_safe() {
            Ok(matches) => matches,
            Err(error) => {
                return Err(error);
            }
        };

        let cwd = match matches.value_of("directory") {
            Some(arg_value) => {
                let path = PathBuf::from(&arg_value);

                let metadata = match metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        return Err(clap::Error::with_description(
                            &format!(
                                "`{}` is not a valid or accessible path: {:?}",
                                arg_value, error
                            ),
                            clap::ErrorKind::InvalidValue,
                        ));
                    }
                };

                if !metadata.is_dir() {
                    return Err(clap::Error::with_description(
                        &format!("`{}` is not a directory", arg_value),
                        clap::ErrorKind::InvalidValue,
                    ));
                }

                path
            }
            None => match current_dir() {
                Ok(path) => path,
                Err(error) => {
                    return Err(clap::Error::with_description(
                        &format!("Can not access current working dir: {:?}", error),
                        clap::ErrorKind::InvalidValue,
                    ));
                }
            },
        };

        let aggresive = matches.is_present("aggresive");

        Ok(Self {
            directory: cwd,
            aggresive,
        })
    }
}
