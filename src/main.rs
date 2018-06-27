extern crate clap;
extern crate colored;
extern crate crossbeam_channel;
#[macro_use]
extern crate lazy_static;
extern crate num_cpus;
extern crate walkdir;

mod byte_format;
mod sqlite_file;

use std::iter::Iterator;
use std::path::PathBuf;
use std::thread;

use byte_format::format_size;
use colored::*;
use crossbeam_channel as channel;
use sqlite_file::{LoadResult, SQLiteFile};
use walkdir::WalkDir;

mod cli_args;

#[derive(Debug)]
enum Status {
    Success(String),
    Error(String),
    Delta(u64),
}

fn start_threads(
    db_file_receiver: channel::Receiver<SQLiteFile>,
    status_sender: channel::Sender<Status>,
) -> Vec<thread::JoinHandle<()>> {
    let cpu_count = num_cpus::get();
    let mut handles: Vec<thread::JoinHandle<_>> = Vec::with_capacity(cpu_count);

    for _ in 0..cpu_count {
        let db_files = db_file_receiver.clone();
        let status = status_sender.clone();

        handles.push(thread::spawn(move || {
            for db_file in db_files {
                match db_file.vacuum() {
                    Ok(result) => {
                        let delta = result.delta();

                        status.send(Status::Delta(delta));
                        status.send(Status::Success(format!(
                            "{} {} {}",
                            "Found".bold().green(),
                            db_file.path().to_str().unwrap_or("?").white(),
                            format_size(delta as f64).yellow().bold(),
                        )));
                    }
                    Err(err) => {
                        status.send(Status::Error(format!("{:?}", err).red().to_string()));
                    }
                };
            }
        }));
    }

    handles
}

// Alias type to avoid verbosity / long lines
type ChannelAPI<T> = (channel::Sender<T>, channel::Receiver<T>);

fn main() {
    let args = match cli_args::Arguments::get() {
        Ok(args) => args,
        Err(error) => {
            error.exit();
        }
    };

    let items = WalkDir::new(&args.directory)
        .into_iter()
        .filter_map(|item| match item {
            Ok(entry) => Some(PathBuf::from(entry.path())),
            Err(_) => None,
        })
        .filter_map(|path| match SQLiteFile::load(&path, args.aggresive) {
            LoadResult::Ok(db_file) => Some(db_file),
            LoadResult::Err(error) => {
                eprintln!("Error reading from `{:?}`: {:?}", &path, error);
                None
            }
            LoadResult::None => None,
        });

    let (file_sender, file_receiver): ChannelAPI<SQLiteFile> = channel::unbounded();
    let (status_sender, status_receiver): ChannelAPI<Status> = channel::unbounded();

    let thread_handles = start_threads(file_receiver, status_sender);

    for mut db_file in items {
        file_sender.send(db_file);
    }
    // Dropping all channel's senders marks it as closed
    drop(file_sender);
    let mut total_delta: u64 = 0;

    for status in status_receiver {
        match status {
            Status::Success(msg) => {
                println!("{}", msg);
            }
            Status::Error(msg) => {
                eprintln!("{}", msg);
            }
            Status::Delta(delta) => {
                total_delta += delta;
            }
        }
    }

    for handle in thread_handles {
        if let Err(error) = handle.join() {
            eprintln!("Thread error: {:?}", error);
        }
    }

    println!(
        "{} {} {}",
        "Done.".bold().bright_green(),
        "Total size reduction:".bright_white(),
        format_size(total_delta as f64).bold().bright_yellow(),
    );
}
