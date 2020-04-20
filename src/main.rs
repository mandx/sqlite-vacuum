mod byte_format;
mod cli_args;
mod display;
mod errors;
mod sqlite_file;

use std::collections::HashMap;
use std::fs::metadata;
use std::iter::Iterator;
use std::path::PathBuf;
use std::thread::{self, JoinHandle};

use console::style;
use crossbeam::channel::{self as channel, Receiver, Sender};
use num_cpus;
use walkdir::WalkDir;

use crate::byte_format::format_size;
use crate::cli_args::Arguments as CliArguments;
use crate::display::Display;
use crate::errors::AppError;
use crate::sqlite_file::SQLiteFile;

#[derive(Debug)]
enum Status {
    Progress(String, i128),
    Error(AppError),
    ErrorMsg(String),
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
                        if let Err(error) = status.send(Status::Progress(
                            format!(
                                "{} {} {}",
                                style("Found").bold().green(),
                                style(db_file.path().to_string_lossy()).white(),
                                style(format_size(delta as f64)).yellow().bold(),
                            ),
                            delta,
                        )) {
                            eprintln!(
                                "Status channel has been closed; Dropping message: {:?}",
                                error
                            );
                        }
                    }
                    Err(error) => {
                        status.send(Status::Error(error)).ok();
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
                        status_sender
                            .send(Status::Error(AppError::not_directory(arg, path)))
                            .ok();
                        None
                    }
                }
                Err(error) => {
                    status_sender
                        .send(Status::Error(AppError::directory_access(error, path)))
                        .ok();
                    None
                }
            });

        for directory in directories {
            for db_file in WalkDir::new(directory)
                .into_iter()
                .filter_map(|item| match item {
                    Ok(entry) => {
                        let path = entry.path();
                        match SQLiteFile::load(path, aggresive) {
                            Ok(Some(db_file)) => Some(db_file),
                            Ok(None) => None,
                            Err(error) => {
                                status_sender.send(Status::Error(error)).ok();
                                None
                            }
                        }
                    }
                    Err(error) => {
                        status_sender
                            .send(Status::ErrorMsg(format!(
                                "Error during directory scan: {:?}",
                                error
                            )))
                            .ok();
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
        Err(error) => error.exit(),
    };

    let cpu_count = num_cpus::get();
    let (file_sender, file_receiver) = channel::bounded(cpu_count);
    let (status_sender, status_receiver) = channel::bounded(cpu_count);

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
    let mut total_delta = 0;

    for status in status_receiver {
        match status {
            Status::Progress(msg, delta) => {
                display.progress(&msg);
                total_delta += delta;
            }
            Status::ErrorMsg(msg) => display.error(&msg),
            Status::Error(error) => {
                display.error(&format!("{}", error));
            }
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
