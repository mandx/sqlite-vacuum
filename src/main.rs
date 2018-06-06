extern crate clap;
extern crate sqlite;
extern crate walkdir;

use std::env::current_dir;
use std::fs::metadata;
use std::io;
use std::iter::Iterator;
use std::path::PathBuf;
use std::process;

use clap::{App, Arg, ArgMatches};
use walkdir::WalkDir;

mod sqlite_file;

fn cli_args<'a>() -> ArgMatches<'a> {
    App::new("sqlite-vacuum")
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
        .get_matches()
}

fn main() -> io::Result<()> {
    let args = cli_args();

    let cwd = match args.value_of("directory") {
        Some(arg_value) => {
            let path = PathBuf::from(&arg_value);
            let metadata = metadata(&path).expect(&format!("`{}` is not a valid path", arg_value));

            if !metadata.is_dir() {
                eprintln!("`{}` is not a directory", arg_value);
                process::exit(1);
            }

            path
        }
        None => current_dir().expect("Can not access current working dir"),
    };

    let aggresive = args.is_present("aggresive");

    let items = WalkDir::new(&cwd)
        .into_iter()
        .filter_map(|item| match item {
            Ok(entry) => Some(PathBuf::from(entry.path())),
            Err(_) => None,
        })
        .filter_map(|path| sqlite_file::SQLiteFile::get(&path, aggresive));

    for mut db_file in items {
        match db_file.vacuum() {
            Ok(_) => {
                println!("Ok {}", db_file);
            }
            Err(error) => {
                println!("Error vacuuming {:?}: {:?}", db_file, error);
            }
        }
    }

    Ok(())
}
