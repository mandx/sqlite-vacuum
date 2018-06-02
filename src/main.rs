extern crate clap;
extern crate sqlite;
extern crate walkdir;

use std::env::current_dir;
use std::fs::{metadata, File};
use std::io;
use std::io::Read;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::process;

use clap::{App, Arg, ArgMatches};
use walkdir::WalkDir;

fn cli_args<'a>() -> ArgMatches<'a> {
    return App::new("sqlite-vacuum")
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
}

fn is_sqlite_db(path: &Path, aggresive: bool) -> bool {
    if aggresive {
        match File::open(path) {
            Ok(file) => {
                let magic: Vec<u8> = vec![
                    0x53, 0x51, 0x4c, 0x69, 0x74, 0x65, 0x20, 0x66, 0x6f, 0x72, 0x6d, 0x61, 0x74,
                    0x20, 0x33, 0x00,
                ];

                let buffer: Vec<u8> = file
                    .bytes()
                    .take(magic.len())
                    .map(|r| r.unwrap_or(0)) // or deal explicitly with failure!
                    .collect();

                buffer == magic
            }
            Err(_) => false,
        }
    } else {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("db") => true,
            Some("sqlite") => true,
            _ => false,
        }
    }
}

fn vacuum(filepath: &Path) -> std::result::Result<(), sqlite::Error> {
    sqlite::open(filepath)
        .and_then(|connection| {
            println!("Connected to {:?}", filepath);
            connection.execute("VACUUM;").and_then(|_| {
                println!("Vacuum'd {:?}", filepath);
                Ok(connection)
            })
        })
        .and_then(|connection| {
            connection.execute("REINDEX;").and_then(|res| {
                println!("Reindexed {:?}", filepath);
                Ok(res)
            })
        })
}

fn main() -> io::Result<()> {
    let args = cli_args();

    let cwd = match args.value_of("directory") {
        Some(arg_value) => {
            let path = PathBuf::from(&arg_value);
            let metadata = metadata(&path).expect(&format!("`{}` is not a valid path", arg_value));

            match metadata.is_dir() {
                true => path,
                false => {
                    eprintln!("`{}` is not a directory", arg_value);
                    process::exit(1);
                }
            }
        }
        None => current_dir().expect("Can not access current working dir"),
    };

    let aggresive = args.is_present("aggresive");

    let paths_iter = WalkDir::new(&cwd)
        .into_iter()
        .filter_map(|item| match item {
            Ok(entry) => Some(PathBuf::from(entry.path())),
            Err(_) => None,
        })
        .filter(|path| is_sqlite_db(&path, aggresive));

    for path in paths_iter {
        match vacuum(&path) {
            Ok(_) => {
                println!("Ok {:?}", path);
            }
            Err(error) => {
                println!("Error vacuuming {:?}: {:?}", path, error);
            }
        }
    }

    Ok(())
}
