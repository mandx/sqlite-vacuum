extern crate clap;
extern crate colored;
#[macro_use]
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

fn start_threads(
    db_file_receiver: channel::Receiver<SQLiteFile>,
    status_sender: channel::Sender<String>,
    error_sender: channel::Sender<String>,
    delta_sender: channel::Sender<u64>,
) -> Vec<thread::JoinHandle<()>> {
    let cpu_count = num_cpus::get();
    let mut handles: Vec<thread::JoinHandle<_>> = Vec::with_capacity(cpu_count);

    for _ in 0..cpu_count {
        let db_files = db_file_receiver.clone();
        let status = status_sender.clone();
        let error = error_sender.clone();
        let delta = delta_sender.clone();

        handles.push(thread::spawn(move || {
            for db_file in db_files {
                match db_file.vacuum() {
                    Ok(result) => {
                        let size_delta = result.delta();

                        delta.send(size_delta);
                        status.send(format!(
                            "{} {} {}",
                            "Found".bold().green(),
                            db_file.path().to_str().unwrap_or("?").white(),
                            format_size(size_delta as f64).yellow().bold(),
                        ));
                    }
                    Err(err) => {
                        error.send(format!("{:?}", err).red().to_string());
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
    let (status_sender, status_receiver): ChannelAPI<String> = channel::unbounded();
    let (error_sender, error_receiver): ChannelAPI<String> = channel::unbounded();
    let (delta_sender, delta_receiver): ChannelAPI<u64> = channel::unbounded();

    let thread_handles = start_threads(file_receiver, status_sender, error_sender, delta_sender);

    for mut db_file in items {
        file_sender.send(db_file);
    }
    // Dropping all channel's senders marks it as closed
    drop(file_sender);
    let mut total_delta: u64 = 0;

    loop {
        // select! {
        //     recv(delta_receiver, msg) => {
        //         if let Some(delta) = msg {
        //             total_delta += delta;
        //         }
        //     },
        //     recv(status_receiver, msg) => {
        //         if let Some(status) = msg {
        //             println!("{}", status);
        //         }
        //     },
        //     recv(error_receiver, msg) => {
        //         if let Some(error) = msg {
        //             eprintln!("{}", error);
        //         }
        //      },

        //     default => { break; },
        // }

        let delta_msg = delta_receiver.recv();
        let status_msg = status_receiver.recv();
        let error_msg = error_receiver.recv();

        match (&delta_msg, &status_msg, &error_msg) {
            (None, None, None) => {
                break;
            },
            _ => {
                if let Some(delta) = delta_msg {
                    total_delta += delta;
                }

                if let Some(status) = status_msg {
                    println!("{}", status);
                }

                if let Some(error) = error_msg {
                    eprintln!("{}", error);
                }
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
