extern crate clap;

use std::env::current_dir;
use std::fs::metadata;
use std::path::PathBuf;

use clap::{App, Arg};
use failure::{Error, ResultExt};

#[derive(Debug)]
pub struct Arguments {
    pub directories: Vec<PathBuf>,
    pub aggresive: bool,
}

impl Arguments {
    pub fn get() -> Result<Self, Error> {
        let app = App::new("sqlite-vacuum")
            .arg(
                Arg::with_name("directory")
                    .value_name("DIRECTORY")
                    .multiple(true)
                    .takes_value(true)
                    .help("Sets the directories to walk")
                    .required(true),
            )
            .arg(
                Arg::with_name("aggresive")
                    .short("a")
                    .long("aggresive")
                    .help("Inspect the file's header to check if it is a SQLite database, instead of just checking the extension (which is faster, but it can lead to false positives).")
                    .takes_value(false)
                    .required(false)
            );

        let matches = app.get_matches_safe()?;

        let directories = match matches.values_of("directory") {
            Some(arg_values) => arg_values
                .filter_map(|value| {
                    let path = PathBuf::from(&value);
                    match metadata(&path).map(|metadata| metadata.is_dir()) {
                        Ok(is_dir) => {
                            if is_dir {
                                Some(path)
                            } else {
                                eprintln!("`{}` is not a directory", value);
                                None
                            }
                        }
                        Err(error) => {
                            eprintln!("`{}` is not a valid directory or it is inaccessible: {:?}", value, error);
                            None
                        }
                    }
                }).collect(),
            None => vec![current_dir().context("Can not access current working dir")?],
        };

        let aggresive = matches.is_present("aggresive");

        Ok(Self {
            directories,
            aggresive,
        })
    }
}
