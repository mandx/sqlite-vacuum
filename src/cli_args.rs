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
    pub fn get() -> Result<Self, String> {
        let matches = App::new("sqlite-vacuum")
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
                    .help("Inspect the file's header to check if it is a SQLite database, instead of just checking the extension. Just checking the extension is faster, but it can lead to false positives.")
                    .takes_value(false)
                    .required(false)
            )
            .get_matches();

        let cwd = match matches.value_of("directory") {
            Some(arg_value) => {
                let path = PathBuf::from(&arg_value);
                let metadata =
                    metadata(&path).expect(&format!("`{}` is not a valid path", arg_value));

                if !metadata.is_dir() {
                    return Err(format!("`{}` is not a directory", arg_value));
                }

                path
            }
            None => current_dir().expect("Can not access current working dir"),
        };

        let aggresive = matches.is_present("aggresive");

        Ok(Self {
            directory: cwd,
            aggresive,
        })
    }
}