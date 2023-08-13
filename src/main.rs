use clap::Parser;
use log::{error, trace};
use std::env;
use std::fs::{DirEntry, ReadDir};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use users::{Users, UsersCache};

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

struct ListParams {
    modes: String,
    usr: String,
    grp: String,
    size: u64,
    name: String,
}

fn main() {
    env_logger::init();
    let args = LsArgs::parse();
    trace!("Starting rsls");

    // current dir or sub dir
    let path_buf = match &args.name {
        Some(name) => PathBuf::from(name),
        None => env::current_dir().expect("Failed to read current dir"),
    };
    trace!(
        "list dir: {:?}",
        path_buf.file_name().expect("Could not name")
    );

    if !path_buf.is_dir() {
        error!("Not a directory");
        std::process::exit(1);
    }
    let dir_entry = read_dir(path_buf, &args);

    match args.list {
        false => list(&dir_entry),
        true => list_info(&dir_entry),
    }
}

fn read_dir(path_buf: PathBuf, args: &LsArgs) -> Vec<DirEntry>{
    /* Get all directory entires */
    let read_dir = path_buf.read_dir().expect("Failed to read directory");

    /* Filter out hidden files if not all argument */
    let mut dir_entry = match args.all {
        false => filter_hidden(read_dir),
        true => read_dir.filter_map(|e| e.ok()).collect(),
    };

    /* Sort DirEntry list */
    dir_entry.sort_by_key(|dir| dir.file_name());
    dir_entry
}

fn filter_hidden(read_dir: ReadDir) -> Vec<DirEntry> {
    /* For each DirEntry, if the name starts with "."
    * return None, else return the entry and collect to a Vec<DirEntry> */
    read_dir
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
        .collect()
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
        let list_params = parse_dir_entry(&e).unwrap();

        println!(
            "{}\t {}\t {}\t {:?}\t {}",
            list_params.modes, list_params.usr, list_params.grp, list_params.size, list_params.name
        );
    }
}

fn parse_dir_entry(e: &DirEntry) -> Result<ListParams, String> {
    let metadata = e.metadata().map_err(|_| String::from("Metadata"))?;

    /* Get name of file */
    let path = e.path();
    let name = path.file_name().ok_or("name")?;
    let name = name.to_str().ok_or("Name")?;

    /* Get permission of file */
    let d = if metadata.is_dir() { "d" } else { "-" };
    let mode = metadata.mode();

    /* A bit ugly, but converting permission to letter */
    let ue = if mode & 0o100 > 0 { "x" } else { "-" };
    let ur = if mode & 0o200 > 0 { "r" } else { "-" };
    let uw = if mode & 0o400 > 0 { "w" } else { "-" };

    let ge = if mode & 0o010 > 0 { "x" } else { "-" };
    let gr = if mode & 0o020 > 0 { "r" } else { "-" };
    let gw = if mode & 0o040 > 0 { "w" } else { "-" };

    let ae = if mode & 0o001 > 0 { "x" } else { "-" };
    let ar = if mode & 0o002 > 0 { "r" } else { "-" };
    let aw = if mode & 0o004 > 0 { "w" } else { "-" };

    let mut modes = String::new();
    modes.push_str(d);
    modes.push_str(ue);
    modes.push_str(ur);
    modes.push_str(uw);
    modes.push_str(ge);
    modes.push_str(gr);
    modes.push_str(gw);
    modes.push_str(ae);
    modes.push_str(ar);
    modes.push_str(aw);

    /* Get user and group of file */
    let usr = metadata.uid();
    let grp = metadata.gid();

    /* Convert uid and gid to name */
    let cache = UsersCache::new();

    let usr = cache.get_user_by_uid(usr).ok_or("Owner")?;
    let usr = usr.name().to_str().ok_or(String::from("Owner"))?;

    let grp = cache.get_user_by_uid(grp).ok_or("Group")?;
    let grp = grp.name().to_str().ok_or(String::from("group"))?;

    /* Get size of file */
    let size = metadata.size();

    let list_params = ListParams {
        modes,
        usr: String::from(usr),
        grp: String::from(grp),
        size,
        name: String::from(name),
    };

    Ok(list_params)
}
