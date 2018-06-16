extern crate clap;
extern crate colored;
extern crate crossbeam_channel;
extern crate num_cpus;
extern crate walkdir;

mod byte_format;
mod sqlite_file;

use std::io;
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
    delta_sender: channel::Sender<u64>,
) -> Vec<thread::JoinHandle<()>> {
    let cpu_count = num_cpus::get();
    let mut handles: Vec<thread::JoinHandle<()>> = Vec::with_capacity(cpu_count);

    for _ in 0..cpu_count {
        let receiver = db_file_receiver.clone();
        let sender = delta_sender.clone();

        handles.push(thread::spawn(move || {
            for db_file in receiver {
                let status = match db_file.vacuum() {
                    Ok(result) => {
                        let delta = result.delta();
                        sender.send(delta);

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
        }));
    }

    handles
}

type ChannelAPI<T> = (channel::Sender<T>, channel::Receiver<T>);

fn main() -> io::Result<()> {
    let args = match cli_args::Arguments::get() {
        Ok(args) => args,
        Err(error_msg) => {
            return Err(io::Error::new(io::ErrorKind::Other, error_msg));
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
    let (delta_sender, delta_receiver): ChannelAPI<u64> = channel::unbounded();

    let thread_handles = start_threads(file_receiver, delta_sender);

    for mut db_file in items {
        file_sender.send(db_file);
    }
    // Dropping all channel's senders marks it as closed
    drop(file_sender);

    for handle in thread_handles {
        if let Err(error) = handle.join() {
            eprintln!("Thread error: {:?}", error);
        }
    }

    let mut total_delta: u64 = 0;
    for delta in delta_receiver {
        total_delta = total_delta + delta;
    }

    println!(
        "{} {} {}",
        "Done.".bold().bright_green(),
        "Total size reduction:".bright_white(),
        format_size(total_delta as f64).bold().bright_yellow(),
    );

    Ok(())
}
