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
use std::sync::{atomic, Arc};
use std::thread;

use byte_format::format_size;
use colored::*;
use crossbeam_channel as channel;
use sqlite_file::{LoadResult, SQLiteFile};
use walkdir::WalkDir;

mod cli_args;

fn start_threads(
    db_file_receiver: &channel::Receiver<SQLiteFile>,
    total_delta: &Arc<atomic::AtomicUsize>,
) -> Vec<thread::JoinHandle<()>> {
    let cpu_count = num_cpus::get();
    let mut handles: Vec<thread::JoinHandle<()>> = Vec::with_capacity(cpu_count);

    for _ in 0..cpu_count {
        let receiver = db_file_receiver.clone();
        let total_delta = total_delta.clone();

        handles.push(thread::spawn(move || {
            for db_file in receiver {
                let status = match db_file.vacuum() {
                    Ok(result) => {
                        let delta = result.delta();
                        total_delta.fetch_add(delta as usize, atomic::Ordering::SeqCst);
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

// Alias type to avoid verbosity / long lines 
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
    let total_delta = Arc::new(atomic::AtomicUsize::new(0));

    let thread_handles = start_threads(&file_receiver, &total_delta);

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

    println!(
        "{} {} {}",
        "Done.".bold().bright_green(),
        "Total size reduction:".bright_white(),
        format_size(total_delta.load(atomic::Ordering::SeqCst) as f64)
            .bold()
            .bright_yellow(),
    );

    Ok(())
}
