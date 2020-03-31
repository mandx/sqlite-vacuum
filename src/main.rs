mod byte_format;
mod cli_args;
mod display;
mod sqlite_file;

use std::collections::HashMap;
use std::fs::metadata;
use std::iter::Iterator;
use std::path::PathBuf;
use std::thread::{self, JoinHandle};

use clap;
use console::style;
use crossbeam::channel::{self as channel, Receiver, Sender};
use num_cpus;
use walkdir::WalkDir;

use crate::byte_format::format_size;
use crate::cli_args::Arguments as CliArguments;
use crate::display::Display;
use crate::sqlite_file::SQLiteFile;

#[derive(Debug)]
enum Status {
    Progress(String),
    Error(String),
    Delta(u64),
}

fn start_threads(
    db_file_receiver: Receiver<SQLiteFile>,
    status_sender: Sender<Status>,
) -> Vec<JoinHandle<()>> {
    let cpu_count = num_cpus::get();
    let mut handles: Vec<JoinHandle<_>> = Vec::with_capacity(cpu_count);

    for _ in 0..cpu_count {
        let db_files = db_file_receiver.clone();
        let status = status_sender.clone();

        handles.push(thread::spawn(move || {
            for db_file in db_files {
                match db_file.vacuum() {
                    Ok(result) => {
                        let delta = result.delta();
                        if let Err(error) = status
                            .send(Status::Progress(format!(
                                "{} {} {}",
                                style("Found").bold().green(),
                                style(db_file.path().to_str().unwrap_or("?")).white(),
                                style(format_size(delta as f64)).yellow().bold(),
                            )))
                            .and_then(|_| status.send(Status::Delta(delta)))
                        {
                            eprintln!(
                                "Status channel has been closed; Dropping message: {:?}",
                                error
                            );
                        }
                    }
                    Err(error) => {
                        if let Err(error) = status.send(Status::Error(format!(
                            "Error vacuuming {}: {:?}",
                            db_file, error
                        ))) {
                            eprintln!(
                                "Status channel has been closed; Dropping message: {:?}",
                                error
                            );
                        }
                    }
                };
            }
        }));
    }

    handles
}

fn start_walking(
    directories: HashMap<String, PathBuf>,
    aggresive: bool,
    status_sender: Sender<Status>,
    file_sender: Sender<SQLiteFile>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let directories = directories
            .iter()
            .filter_map(|(arg, path)| match metadata(&path) {
                Ok(meta) => {
                    if meta.is_dir() {
                        Some(path)
                    } else {
                        if let Err(error) = status_sender
                            .send(Status::Error(format!("`{}` is not a directory", arg)))
                        {
                            eprintln!(
                                "Status channel has been closed; Dropping message: {:?}",
                                error
                            );
                        }
                        None
                    }
                }
                Err(error) => {
                    if let Err(error) = status_sender.send(Status::Error(format!(
                        "`{}` is not a valid directory or it is inaccessible: {:?}",
                        arg, error
                    ))) {
                        eprintln!(
                            "Status channel has been closed; Dropping message: {:?}",
                            error
                        );
                    }
                    None
                }
            });

        for directory in directories {
            for db_file in WalkDir::new(directory)
                .into_iter()
                .filter_map(|item| match item {
                    Ok(entry) => {
                        let path = entry.path();
                        if let Some(filename) = path.to_str() {
                            if let Err(error) =
                                status_sender.send(Status::Progress(filename.into()))
                            {
                                eprintln!(
                                    "Status channel has been closed; Dropping message: {:?}",
                                    error
                                );

                                return None;
                            }
                        }

                        match SQLiteFile::load(path, aggresive) {
                            Ok(Some(db_file)) => Some(db_file),
                            Ok(None) => None,
                            Err(error) => {
                                if let Err(error) = status_sender.send(Status::Error(format!(
                                    "Error reading from `{:?}`: {:?}",
                                    &path, error
                                ))) {
                                    eprintln!(
                                        "Status channel has been closed: Dropping message: {:?}",
                                        error
                                    );
                                }

                                None
                            }
                        }
                    }
                    Err(error) => {
                        if let Err(error) = status_sender.send(Status::Error(format!(
                            "Error during directory scan: {:?}",
                            error
                        ))) {
                            eprintln!(
                                "Status channel has been closed: Dropping message: {:?}",
                                error
                            );
                        }

                        None
                    }
                })
            {
                if let Err(error) = file_sender.send(db_file) {
                    eprintln!(
                        "Worker channel has been closed; Stopping directory enumeration: {:?}",
                        error
                    );

                    return;
                }
            }
        }
    })
}

fn main() {
    let args = match CliArguments::get() {
        Ok(arguments) => arguments,
        Err(error) => match error.downcast::<clap::Error>() {
            Ok(clap_error) => clap_error.exit(),
            Err(other_error) => {
                eprintln!("Error parsing arguments: {:?}", other_error);
                std::process::exit(1);
            }
        },
    };

    let (file_sender, file_receiver) = channel::unbounded();
    let (status_sender, status_receiver) = channel::unbounded();

    let threads = {
        let mut threads = start_threads(file_receiver, status_sender.clone());
        threads.push(start_walking(
            args.directories.clone(),
            args.aggresive,
            status_sender,
            file_sender,
        ));
        threads
    };

    let display = Display::new();
    let mut total_delta: u64 = 0;

    for status in status_receiver {
        match status {
            Status::Progress(msg) => display.progress(&msg),
            Status::Error(msg) => display.error(&msg),
            Status::Delta(delta) => total_delta += delta,
        }
    }

    for handle in threads {
        if let Err(error) = handle.join() {
            display.error(&format!("Thread error: {:?}", error));
        }
    }

    display.write_line(&format!(
        "{} {} {}",
        style("Done.").bold().green(),
        style("Total size reduction:").white(),
        style(format_size(total_delta as f64)).bold().yellow(),
    ));
}
