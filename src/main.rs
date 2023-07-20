use std::env;
use std::fs::DirEntry;
use std::os::unix::fs::MetadataExt;
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
                        .map(|s| s.starts_with("."))
                        .unwrap()
                    {
                        true => {
                            trace!("Filter hidden file{:?}", entry.file_name());
                            None
                        }
                        false => Some(entry),
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
    println!("Mode\t\t user\t group\t size\t name");
    for e in entries.iter() {
        if let Ok(metadata) = e.metadata() {
            // TODO: Change this
            if let Some(name) = e.path().file_name() {
                let name = name.to_str().expect("OS String failed to be converted");
                let d = if metadata.is_dir() { "d" } else { "-" };
                let size = metadata.size();
                let mode = metadata.mode();
                let usr = metadata.uid(); // TODO: Convert to text
                let grp = metadata.gid(); // TODO: Convert to text

                let ue = if mode & 0o100 > 0 { "x" } else { "-" };
                let ur = if mode & 0o200 > 0 { "r" } else { "-" };
                let uw = if mode & 0o400 > 0 { "w" } else { "-" };

                let ge = if mode & 0o010 > 0 { "x" } else { "-" };
                let gr = if mode & 0o020 > 0 { "r" } else { "-" };
                let gw = if mode & 0o040 > 0 { "w" } else { "-" };

                let ae = if mode & 0o001 > 0 { "x" } else { "-" };
                let ar = if mode & 0o002 > 0 { "r" } else { "-" };
                let aw = if mode & 0o004 > 0 { "w" } else { "-" };

                println!(
                    "{d}{ur}{uw}{ue}{gr}{gw}{ge}{ar}{aw}{ae}\t {:?}\t {:?}\t {:?}\t {}",
                    usr, grp, size, name
                );
            }
        }
    }
}
