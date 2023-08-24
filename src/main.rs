use clap::Parser;
use crossterm::{queue, style};
use log::{error, trace, warn};
use std::env;
use std::fs::{DirEntry, Metadata, ReadDir};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::OnceLock;
use users::{Users, UsersCache};

#[derive(Clone, Parser, Debug)]
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

    /// List files in a tree
    #[arg(short, long, default_value_t = 1)]
    tree: u8,
}

static ARGS: OnceLock<LsArgs> = OnceLock::new();
static ITEM_SIGN: &str = "|-";
static LAST_SIGN: &str = "|_";

struct FSFile {
    name: String,
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

fn main() {
    env_logger::init();
    let args = ARGS.get_or_init(LsArgs::parse);

    trace!("Starting rsls");

    // current dir or sub dir
    let path_buf = match &args.name {
        Some(name) => PathBuf::from(name),
        None => env::current_dir().expect("Failed to read current dir"),
    };

    trace!(
        "list dir: {:?}",
        path_buf.file_name().expect("Could not read directory name")
    );

    if !path_buf.is_dir() {
        error!("Not a directory");
        std::process::exit(1);
    }

    let dir_entry = read_directory(path_buf, args.tree).expect("Failed to read dir");

    // TODO: Fix listing based on new type
    list(&dir_entry).expect("Failed to print stuff");
}

fn read_directory(path: PathBuf, depth: u8) -> Result<FSFile, String> {
    /* TODO: Verify it is a dir */
    trace!("Read dir: {:?}", path);

    let args = ARGS.get().ok_or("Failed to read settings")?;
    let mut dir_type = DirType { childs: Vec::new() };
    let name = path
        .file_name()
        .ok_or(String::from("failed to read file name"))?
        .to_str()
        .ok_or(String::from("Failed to read file name"))?;
    let metadata = path
        .metadata()
        .map_err(|_| String::from("Failed to open metadata"))?;

    if depth == 0 {
        /* Do not read dir, just return the FSObject */
        return Ok(FSFile {
            name: String::from(name),
            path_buf: path.clone(),
            metadata,
            entry_type: FSFileType::Dir(dir_type),
        });
    }

    /* Get all directory entires */
    let read_dir = path
        .read_dir()
        .map_err(|_| String::from("Failed to read dir"))?;

    /* Filter out hidden files if not all argument */
    let dir_entry = match args.all {
        false => filter_hidden(read_dir),
        true => read_dir.filter_map(|e| e.ok()).collect(),
    };

    for d in dir_entry {
        if let Ok(subdir_metadata) = d.metadata() {
            if subdir_metadata.is_dir() {
                /* Recursivley read directory */
                if let Ok(sub_dir) = read_directory(d.path(), depth - 1) {
                    /* Store read directory in current directory */
                    dir_type.childs.push(sub_dir);
                }
            } else if subdir_metadata.is_file() {
                /* Store the file */
                if let Some(subdir_name) = d.file_name().to_str() {
                    let fs_file = FSFile {
                        name: String::from(subdir_name),
                        path_buf: d.path(),
                        metadata: subdir_metadata,
                        entry_type: FSFileType::File,
                    };
                    dir_type.childs.push(fs_file);
                }
            } else if subdir_metadata.is_symlink() {
                /* Store the symlink */
                trace!("Symlink, should display it in better color");
            }
        } else {
            warn!("Failed to open meta data of child dir.");
        }
    }

    /* Sort childs */
    dir_type
        .childs
        .sort_by_key(|fs_file| fs_file.path_buf.clone());
    Ok(FSFile {
        name: String::from(name),
        path_buf: path.clone(),
        metadata,
        entry_type: FSFileType::Dir(dir_type),
    })
}

fn filter_hidden(read_dir: ReadDir) -> Vec<DirEntry> {
    /* For each DirEntry, if the name starts with "."
     * return None, else return the entry and collect to a Vec<DirEntry> */
    read_dir
        .filter_map(|res_entry| {
            res_entry.ok().and_then(|entry| {
                match entry.file_name().to_str().map(|s| s.starts_with('.')) {
                    Some(false) => Some(entry),
                    Some(true) => {
                        trace!("Filter hidden file{:?}", entry.file_name());
                        None
                    }
                    None => {
                        error!("error filtering hidden files");
                        None
                    }
                }
            })
        })
        .collect()
}

fn list(fs_file: &FSFile) -> Result<(), String> {
    let mut stdout = std::io::stdout();
    let args = ARGS.get().unwrap();
    if args.list && args.tree > 1 {
        warn!("Cannot display list and tree, will do list");
        println!("Mode\t\t user\t group\t size\t\t name"); // TODO: Convert to print
        print_dir(&mut stdout, fs_file)?;
        queue!(stdout, style::Print("\n")).map_err(|_| String::from("Failed to print name"))?;
        let _ = stdout.flush();
        return Ok(());
    }

    if args.list {
        println!("Mode\t\t user\t group\t size\t\t name"); // TODO: Convert to print
    }

    if args.tree > 1 {
        print_dir_rec(&mut stdout, fs_file, 0)?;
    } else {
        print_dir(&mut stdout, fs_file)?;
    }

    queue!(stdout, style::Print("\n")).map_err(|_| String::from("Failed to print name"))?;
    let _ = stdout.flush();
    Ok(())
}

/// Prints a single directory
fn print_dir<W>(w: &mut W, fs_file: &FSFile) -> Result<(), String>
where
    W: std::io::Write,
{
    let args = ARGS.get().unwrap();
    match &fs_file.entry_type {
        FSFileType::Dir(dir_type) => {
            for child in dir_type.childs.iter() {
                let output_string = if args.list {
                    parse_dir_entry(child)?
                } else {
                    let mut n = String::with_capacity(64);
                    n.push_str(child.name.as_str());
                    n.push('\t');
                    n
                };
                queue!(w, style::Print(output_string.to_string()))
                    .map_err(|_| String::from("Failed to print name"))?;
            }
        }
        FSFileType::File => {
            error!("Cannot list file");
        }
    }
    Ok(())
}

/// NOTE! Incompatible with -l command due to visualization problems
fn print_dir_rec<W>(w: &mut W, fs_file: &FSFile, depth: u8) -> Result<(), String>
where
    W: std::io::Write,
{
    let args = ARGS.get().ok_or("Failet to get settings")?;
    if args.tree <= depth {
        return Ok(());
    }
    let indent = (0..depth).map(|_| "|  ").collect::<String>();
    match &fs_file.entry_type {
        FSFileType::Dir(dir_type) => {
            let mut it = dir_type.childs.iter().peekable();
            while let Some(child) = it.next() {
                let last = it.peek().is_none();
                match &child.entry_type {
                    FSFileType::File => {
                        // TODO: refactor to its own function
                        let mut prefix = indent.clone();
                        if last {
                            prefix.push_str(LAST_SIGN);
                        } else {
                            prefix.push_str(ITEM_SIGN);
                        };

                        queue!(
                            w,
                            style::Print(format!("{}{}\n", prefix.clone(), child.name.clone()))
                        )
                        .map_err(|_| String::from("Failed to print name"))?;
                    }
                    FSFileType::Dir(c) => {
                        // TODO: refactor to its own function
                        let mut prefix = indent.clone();
                        if last && c.childs.is_empty() {
                            prefix.push_str(LAST_SIGN);
                        } else {
                            prefix.push_str(ITEM_SIGN);
                        };
                        queue!(
                            w,
                            style::Print(format!("{}{}\n", prefix.clone(), child.name.clone()))
                        )
                        .map_err(|_| String::from("Failed to print name"))?;
                        let _ = print_dir_rec(w, child, depth + 1);
                    }
                }
            }
        }
        _ => {
            error!("Cannot list non dir type");
        }
    }
    Ok(())
}

fn parse_dir_entry(fs_file: &FSFile) -> Result<String, String> {
    let metadata = fs_file.metadata.clone();

    /* Get name of file */
 // Can fail due to permissions, symbolic link or path errors.
    let name = fs_file.name.clone();

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

    let mut modes = String::with_capacity(64);
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

    modes.push('\t');
    modes.push_str(usr);
    modes.push('\t');
    modes.push_str(grp);
    modes.push('\t');
    modes.push_str(size.to_string().as_str());
    modes.push('\t');
    modes.push('\t');
    modes.push_str(name.as_str());
    modes.push('\n');

    Ok(modes)
}
