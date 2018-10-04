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
use sqlite_file::SQLiteFile;
use walkdir::WalkDir;

mod cli_args;
mod display;

#[derive(Debug)]
enum Status {
    Progress(String),
    Error(String),
    Delta(u64),
}

// We do want this function to consume/own its parameters
#[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
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

fn main() {
    let args = match cli_args::Arguments::get() {
        Ok(arguments) => arguments,
        Err(error) => match error.downcast::<clap::Error>() {
            Ok(clap_error) => clap_error.exit(),
            Err(other_error) => {
                eprintln!("Error parsing arguments: {:?}", other_error);
                std::process::exit(1);
            }
        },
    };

    let display = display::Display::new();

    let (file_sender, file_receiver) = channel::unbounded();
    let (status_sender, status_receiver) = channel::unbounded();

    let thread_handles = start_threads(file_receiver, status_sender.clone());

    for directory in &args.directories {
        WalkDir::new(directory)
            .into_iter()
            .filter_map(|item| match item {
                Ok(entry) => {
                    let path = entry.path();
                    if let Some(filename) = path.to_str() {
                        display.progress(filename);
                    }
                    Some(PathBuf::from(path))
                }
                Err(error) => {
                    status_sender.send(Status::Error(
                        style(format!("Error during directory scan: {:?}", error))
                            .red()
                            .to_string(),
                    ));
                    None
                }
            }).filter_map(|path| match SQLiteFile::load(&path, args.aggresive) {
                Ok(Some(db_file)) => Some(db_file),
                Ok(None) => None,
                Err(error) => {
                    status_sender.send(Status::Error(
                        style(format!("Error reading from `{:?}`: {:?}", &path, error))
                            .red()
                            .to_string(),
                    ));
                    None
                }
            }).for_each(|db_file| file_sender.send(db_file));
    }

    drop(file_sender);
    drop(status_sender);

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
            display.error(
                &style(format!("Thread error: {:?}", error))
                    .red()
                    .to_string(),
            );
        }
    }

    display.write_line(&format!(
        "{} {} {}",
        style("Done.").bold().green(),
        style("Total size reduction:").white(),
        style(format_size(total_delta as f64)).bold().yellow(),
    ));
}
