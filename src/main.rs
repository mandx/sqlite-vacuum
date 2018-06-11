extern crate clap;
extern crate colored;
extern crate sqlite;
extern crate walkdir;

mod byte_format;
mod sqlite_file;

use std::env::current_dir;
use std::fs::metadata;
use std::io;
use std::iter::Iterator;
use std::path::PathBuf;
use std::process;

use byte_format::format_size;
use clap::{App, Arg, ArgMatches};
use colored::*;
use walkdir::WalkDir;

use sqlite_file::SQLiteFile;

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
        .filter_map(|path| SQLiteFile::get(&path, aggresive));

    let mut total_delta: u64 = 0;

    for mut db_file in items {
        let status = match db_file.vacuum() {
            Ok(result) => {
                let delta = result.delta();
                total_delta = total_delta + delta;
                format_size(delta as f64).yellow()
            }
            Err(error) => format!("{:?}", error).red(),
        };

        println!(
            "{} {} {}",
            "Found".bold().green(),
            db_file.path().to_str().unwrap_or("?").white(),
            status.bold(),
        );
    }

    println!(
        "{} {} {}",
        "Done.".bold().bright_green(),
        "Total size reduction:".bright_white(),
        format_size(total_delta as f64).bold().bright_yellow(),
    );

    Ok(())
}
