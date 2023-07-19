use std::env;
use std::fs::DirEntry;
use std::path::PathBuf;

use clap::Parser;
use log::{error, trace};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct LsArgs {
    /// Directory to list
    name: Option<String>,

    /// List files
    #[arg(short, long)]
    list: bool,

    /// List all files
    #[arg(short, long)]
    all: bool,
}

fn main() {
    env_logger::init();
    let args = LsArgs::parse();
    trace!("Starting rsls");

    // current dir or sub dir
    let path_buf = match args.name {
        Some(name) => PathBuf::from(name),
        None => env::current_dir().expect("Failed to read current dir"),
    };
    trace!(
        "list dir: {:?}",
        path_buf.file_name().expect("Could not name")
    );

    if !path_buf.is_dir() {
        error!("Not a directory");
    }

    /* Get all directory entires */
    let read_dir = path_buf.read_dir().expect("Failed to read directory");

    /* Filter out hidden files if not all argument */
    let dir_entry = match args.all {
        false => read_dir
            .filter_map(|res_entry| {
                res_entry.ok().and_then(|entry| {
                    match entry
                        .file_name()
                        .to_str()
                        .map(|s| s.starts_with(".")).unwrap()
                    {
                        true => {
                                None
                            },
                        false => {
                                Some(entry)
                            }
                    }
                })
            })
            .collect(),
        true => read_dir.filter_map(|e| e.ok()).collect(),
    };

    match args.list {
        false => list(&dir_entry),
        true => list_info(&dir_entry),
    }
}

fn list(entries: &Vec<DirEntry>) {
    for e in entries.iter() {
        if let Some(name) = e
            .path()
            .file_name()
            .expect("failed to read file name")
            .to_str()
        {
            print!("{name}\t");
        }
    }
    println!();
}

fn list_info(entries: &Vec<DirEntry>) {
    for e in entries.iter() {
        if let Some(name) = e
            .path()
            .file_name()
            .expect("failed to read file name")
            .to_str()
        {
            println!("{name}");
        }
    }
}
