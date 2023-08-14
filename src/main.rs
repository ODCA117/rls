use clap::Parser;
use log::{error, trace};
use std::env;
use std::fs::{DirEntry, ReadDir, Metadata};
use std::path::PathBuf;
use std::os::unix::fs::MetadataExt;
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

    /// List files recursive
    #[arg(short, long, default_value_t = 1)]
    recursive: u8,
}

struct FSFile {
    path_buf: PathBuf,
    metadata: Metadata,
    entry_type: FSFileType,
}

enum FSFileType {
    File,
    Dir(DirType),
}

struct DirType {
    pub childs: Vec<FSFile>,
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

    let dir_entry = read_directory(path_buf, args.recursive, &args);

    // TODO: Fix listing based on new type
    match args.list {
        false => list(&dir_entry),
        true => list_info(&dir_entry),
    }
}

fn read_directory(path: PathBuf, depth: u8, args: &LsArgs) -> FSFile {
    /* TODO: Verify it is a dir */

    trace!("Read dir: {:?}", path);

    let mut dir_type = DirType {childs: Vec::new()};

    if depth <= 0 {
        /* Do not read dir, just return the FSObject */
        return FSFile {path_buf: path.clone(), metadata: path.metadata().unwrap(), entry_type: FSFileType::Dir(dir_type)};
    }

    /* Get all directory entires
    * TODO: Fix error handling */
    let read_dir = path.read_dir().expect("Failed to read directory");

    /* Filter out hidden files if not all argument */
    let dir_entry = match args.all {
        false => filter_hidden(read_dir),
        true => read_dir.filter_map(|e| e.ok()).collect(),
    };

    for d in dir_entry {
        /* Fix error handling */
        let metadata = d.metadata().expect("Could not fetch metadata");

        if metadata.is_dir() {
            /* Recursivley read directory */
            let sub_dir = read_directory(d.path(), depth - 1, &args);
            /* Store read directory in current directory */
            dir_type.childs.push(sub_dir);
        }
        else if metadata.is_file() {
            /* Store the file */
            let fs_file = FSFile {path_buf: d.path(), metadata, entry_type: FSFileType::File};
            dir_type.childs.push(fs_file);
        }
        else if metadata.is_symlink() {
            /* Store the symlink */
            trace!("Symlink, should display it in better color");
        }
    }

    /* Sort childs */
    dir_type.childs.sort_by_key(|fs_file| fs_file.path_buf.clone());
    return FSFile {path_buf: path.clone(), metadata: path.metadata().unwrap(), entry_type: FSFileType::Dir(dir_type)};
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

fn list(fs_file: &FSFile) {
    match &fs_file.entry_type {
        FSFileType::Dir(dir_type) => {
            for child in dir_type.childs.iter() {
                print!("{}\t", child.path_buf.file_name().unwrap().to_str().unwrap());
                match &child.entry_type {
                    FSFileType::File => continue,
                    FSFileType::Dir(_) => {
                        list(child);
                    },
                }
            }
        },
        FSFileType::File => {
            error!("Cannot list file");
        },
    }
    println!();
}

fn list_info(fs_file: &FSFile) {
    println!("Mode\t\t user\t group\t size\t name");
    match &fs_file.entry_type {
        FSFileType::Dir(dir) => {
            for child in dir.childs.iter() {
                let list_params = parse_dir_entry(child).unwrap();
                println!(
                    "{}\t {}\t {}\t {:?}\t {}",
                    list_params.modes, list_params.usr, list_params.grp, list_params.size, list_params.name
                );
                match &child.entry_type {
                    FSFileType::File => continue,
                    FSFileType::Dir(_) => {
                        list_info(child);
                    },
                }
            }
        },
        FSFileType::File => {
            error!("Cannot list file");
        },
    }
}

fn parse_dir_entry(fs_file: &FSFile) -> Result<ListParams, String> {

    let metadata = fs_file.metadata.clone();

    /* Get name of file */
    let name = fs_file.path_buf.file_name().unwrap().to_str().unwrap();

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
