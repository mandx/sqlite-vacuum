extern crate atty;
extern crate clap;
extern crate console;
extern crate crossbeam_channel;
#[macro_use]
extern crate failure;
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
use console::style;
use crossbeam_channel as channel;
use sqlite_file::{SQLiteFile};
use walkdir::WalkDir;

mod cli_args;
mod display;
mod errors;

// use errors::*;

#[derive(Debug)]
enum Status {
    Progress(String),
    Error(String),
    Delta(u64),
}

fn start_threads(
    db_file_receiver: &channel::Receiver<SQLiteFile>,
    status_sender: &channel::Sender<Status>,
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
                        status.send(Status::Progress(format!(
                            "{} {} {}",
                            style("Found").bold().green(),
                            style(db_file.path().to_str().unwrap_or("?")).white(),
                            style(format_size(delta as f64)).yellow().bold(),
                        )));
                    }
                    Err(err) => {
                        status.send(Status::Error(style(format!("{:?}", err)).red().to_string()));
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
        Err(error) => match error.downcast::<clap::Error>() {
            Ok(clap_error) => clap_error.exit(),
            Err(other_error) => {
                eprintln!("Error parsing arguments: {:?}", other_error);
                std::process::exit(1);
            }
        },
    };

    let display = display::Display::new();

    let items = WalkDir::new(&args.directory)
        .into_iter()
        .filter_map(|item| match item {
            Ok(entry) => Some(PathBuf::from(entry.path())),
            Err(_) => None,
        })
        .filter_map(|path| match SQLiteFile::load(&path, args.aggresive) {
            Some(Ok(db_file)) => Some(db_file),
            Some(Err(error)) => {
                let msg = format!("Error reading from `{:?}`: {:?}", &path, error);
                display.error(&style(msg).red().to_string());
                None
            }
            None => None,
        });

    let (file_sender, file_receiver): ChannelAPI<SQLiteFile> = channel::unbounded();
    let (status_sender, status_receiver): ChannelAPI<Status> = channel::unbounded();

    let thread_handles = start_threads(&file_receiver, &status_sender);

    for mut db_file in items {
        file_sender.send(db_file);
    }

    // Manually drop the senders, so their respective channels are
    // set as closed. Otherwise all receivers will block.
    drop(status_sender);
    drop(file_sender);
    // We could have the `start_threads` function consume its parameters
    // (move the values, instead of taking borrowing references), but
    // then Clippy complains about unnecessary allocations something something...
    // See https://rust-lang-nursery.github.io/rust-clippy/v0.0.211/index.html#needless_pass_by_value

    let mut total_delta: u64 = 0;

    for status in status_receiver {
        match status {
            Status::Progress(msg) => display.progress(&msg),
            Status::Error(msg) => display.error(&msg),
            Status::Delta(delta) => total_delta += delta,
        }
    }

    for handle in thread_handles {
        if let Err(error) = handle.join() {
            display.error(&style(format!("Thread error: {:?}", error))
                .red()
                .to_string());
        }
    }

    display.write_line(&format!(
        "{} {} {}",
        style("Done.").bold().green(),
        style("Total size reduction:").white(),
        style(format_size(total_delta as f64)).bold().yellow(),
    ));
}
