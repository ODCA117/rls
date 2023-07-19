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
}

fn main() {
    env_logger::init();
    let args = LsArgs::parse();
    trace!("Starting rsls");
    trace!("list: {}", args.list);

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

    let dir_entry = path_buf
        .read_dir()
        .expect("Failed to read directory")
        .filter_map(|e| e.ok())
        .collect();

    match args.list {
        true => list(&dir_entry),
        false => list_info(&dir_entry),
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
            println!("{name}");
        }
    }
}

fn list_info(entries: &Vec<DirEntry>) {
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
}
