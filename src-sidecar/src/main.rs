use std::{collections::HashMap, path::Path, time::Instant};

use clap::{Arg, ArgAction, Command, value_parser};
use directories::BaseDirs;
use jwalk::WalkDir;
use runas::Command as RunasCommand;
use rusqlite::Connection;
use src_lib::{IndexOptions, get_drive_letter};
use usn_journal_rs::{mft::Mft, path::MftPathResolver, volume::Volume};

fn main() {
    let matches = Command::new("Indexer")
        .version("1.0")
        .arg(Arg::new("path").long("path").value_parser(value_parser!(String)).help("Path to index").required(true))
        .arg(Arg::new("index").long("index").value_parser(value_parser!(String)).help("Index files and/or directories (comma-separated) - files/dirs)").required(true))
        .arg(Arg::new("types").long("types").value_parser(value_parser!(String)).help("File types to index (comma-separated)"))
        .arg(Arg::new("exclude").long("exclude").value_parser(value_parser!(String)).help("Exclude common unwanted files (comma-separated) - empty, temp, hidden, system, privileged"))
        .arg(Arg::new("exclude_regex").long("exclude-regex").value_parser(value_parser!(String)).help("Exclude files or directories matching this regex pattern"))
        .arg(Arg::new("exclude_paths").long("exclude-paths").value_parser(value_parser!(String)).help("Exclude specific paths (comma-separated)"))
        .arg(Arg::new("exclude_files").long("exclude-files").value_parser(value_parser!(String)).help("Exclude specific files (comma-separated)"))
        .arg(Arg::new("elevate").long("elevate").help("Request elevation").action(ArgAction::SetTrue))
        .get_matches();

    if matches.get_flag("elevate") {
        // get path of executable and rerun self as admin
        let path = std::env::current_exe().expect("Could not get current executable path");
        let mut command = RunasCommand::new(&path);
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        command.args(&args);
        command
            .status()
            .expect("Failed to execute elevated command");
        std::process::exit(0); // exit current process
    }

    println!("path: {}", matches.get_one::<String>("path").unwrap());
    println!("indexing: {}", matches.get_one::<String>("index").unwrap());
    if let Some(types) = matches.get_one::<String>("types") {
        println!("types: {}", types);
    }
    if let Some(exclude_regex) = matches.get_one::<String>("exclude_regex") {
        println!("exclude_regex: {}", exclude_regex);
    }
    if let Some(exclude_paths) = matches.get_one::<String>("exclude_paths") {
        println!("exclude_paths: {}", exclude_paths);
    }
    if let Some(exclude_files) = matches.get_one::<String>("exclude_files") {
        println!("exclude_files: {}", exclude_files);
    }
    if let Some(exclude) = matches.get_one::<String>("exclude") {
        println!("exclude: {}", exclude);
    }
    if matches.get_flag("elevate") {
        println!("elevate: true");
    }

    let basedirs = BaseDirs::new().expect("Could not get base dirs");
    let app_data_dir = basedirs.data_dir().join("me.jovannmc.guessfs");
    println!("app data dir: {}", &app_data_dir.display());
    let index_options = IndexOptions {
        path: matches.get_one::<String>("path").unwrap().to_string(),
        index_directories: matches
            .get_one::<String>("index")
            .map(|s| s.split(',').any(|x| x == "dirs"))
            .unwrap_or(false),
        index_files: matches
            .get_one::<String>("index")
            .map(|s| s.split(',').any(|x| x == "files"))
            .unwrap_or(false),
        file_types: matches
            .get_one::<String>("types")
            .map(|s| Some(s.split(',').map(String::from).collect::<Vec<String>>()))
            .unwrap_or(None),
        excluded_regex: matches.get_one::<String>("exclude_regex").map(String::from),
        excluded_paths: matches
            .get_one::<String>("exclude_paths")
            .map(|s| s.split(',').map(String::from).collect())
            .map_or_else(|| Some(Vec::new()), |v| Some(v)),
        excluded_files: matches
            .get_one::<String>("exclude_files")
            .map(|s| s.split(',').map(String::from).collect())
            .map_or_else(|| Some(Vec::new()), |v| Some(v)),
        exclude_hidden: matches
            .get_one::<String>("exclude")
            .map_or(Some(false), |s| Some(s.split(',').any(|x| x == "hidden"))),
        exclude_system: matches
            .get_one::<String>("exclude")
            .map_or(Some(false), |s| Some(s.split(',').any(|x| x == "system"))),
        exclude_temporary: matches
            .get_one::<String>("exclude")
            .map_or(Some(false), |s| Some(s.split(',').any(|x| x == "temp"))),
        exclude_empty: matches
            .get_one::<String>("exclude")
            .map_or(Some(false), |s| Some(s.split(',').any(|x| x == "empty"))),
        exclude_admin: matches
            .get_one::<String>("exclude")
            .map_or(Some(false), |s| {
                Some(s.split(',').any(|x| x == "privileged"))
            }),
    };

    start_indexing(&app_data_dir, index_options)
        .map(|result| println!("{}", result))
        .unwrap_or_else(|err| eprintln!("Error: {}", err));
}

fn start_indexing(app_data_dir: &Path, index_options: IndexOptions) -> Result<String, String> {
    let mut exclude_counts: HashMap<&'static str, usize> = HashMap::new();

    let db_path = src_lib::get_index_db_path(app_data_dir, &index_options.path)?;

    let mut db =
        Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    if !db_path.exists() {
        src_lib::init_db(&db).map_err(|e| format!("Failed to initialize database: {}", e))?;
        println!("New database created at: {}", db_path.display());
    } else {
        src_lib::init_db(&db).map_err(|e| format!("Failed to initialize database: {}", e))?;
        println!("Database already exists at: {}", db_path.display());
    }

    let path = Path::new(&index_options.path);
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", index_options.path));
    }

    let transaction = db
        .transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    let start_time = Instant::now();
    let mut folders_found = 0;
    let mut files_found = 0;
    let mut ignored = 0;
    let mut exists = 0;

    let is_root = path.components().count() == 2; // check if the path is a root directory (e.g., C:\) - C: and \ counts as two components
    let is_ntfs = src_lib::is_ntfs(&path);

    // Non-NTFS filesystem / not Windows
    // also check if the path is not a root directory, because with MFT we can only index the entire root
    if !is_ntfs || (is_ntfs && !is_root) {
        println!("Not using NTFS MFT for path: {}", index_options.path);
        // prepare folder and file insert statements
        let mut folder_stmt = transaction
            .prepare("INSERT OR IGNORE INTO folders (path) VALUES (?1)")
            .unwrap();
        let mut file_stmt = transaction
            .prepare("INSERT OR IGNORE INTO files (path, folder_id) VALUES (?1, ?2)")
            .unwrap();
        // build a map to query instead of querying the DB each time
        let mut folder_map = std::collections::HashMap::new();
        for entry in WalkDir::new(path) {
            match entry {
                Ok(entry) => {
                    // check if needed to be excluded
                    if src_lib::should_exclude(
                        &entry.path(),
                        index_options.clone(),
                        &mut exclude_counts,
                    ) {
                        ignored += 1;
                        continue;
                    }

                    // if directory, insert
                    if entry.file_type().is_dir() {
                        if let Some(path) = entry.path().to_str() {
                            let rows = folder_stmt.execute(rusqlite::params![path]).unwrap();
                            if rows > 0 {
                                folders_found += 1
                            } else {
                                // Already exists in DB
                                exists += 1;
                            }
                            if !folder_map.contains_key(path) {
                                let id: i64 = transaction
                                    .query_row(
                                        "SELECT id FROM folders WHERE path = ?1",
                                        rusqlite::params![path],
                                        |row| row.get(0),
                                    )
                                    .unwrap();
                                folder_map.insert(path.to_string(), id);
                            }
                        } else {
                            eprintln!("Path not UTF-8: {:?}", entry.path());
                        }
                    // if user wants to index files, insert them too
                    } else if index_options.index_files {
                        if let Some(path) = entry.path().to_str() {
                            if let Some(parent) =
                                std::path::Path::new(path).parent().and_then(|p| p.to_str())
                            {
                                if let Some(folder_id) = folder_map.get(parent) {
                                    let rows = file_stmt
                                        .execute(rusqlite::params![path, folder_id])
                                        .unwrap();
                                    if rows > 0 {
                                        files_found += 1
                                    } else {
                                        // Already exists in DB
                                        exists += 1;
                                    }
                                } else {
                                    // if not in map, fetch from DB
                                    if let Ok(folder_id) = transaction.query_row(
                                        "SELECT id FROM folders WHERE path = ?1",
                                        rusqlite::params![parent],
                                        |row| row.get(0),
                                    ) {
                                        folder_map.insert(parent.to_string(), folder_id);
                                        let rows = file_stmt
                                            .execute(rusqlite::params![path, &folder_id])
                                            .unwrap();
                                        if rows > 0 {
                                            files_found += 1
                                        } else {
                                            // Already exists in DB
                                            exists += 1;
                                        }
                                    } else {
                                        // not found, create parent folder in DB
                                        let rows =
                                            folder_stmt.execute(rusqlite::params![parent]).unwrap();
                                        if rows > 0 {
                                            folders_found += 1
                                        } else {
                                            // Already exists in DB
                                            exists += 1;
                                        }
                                    }
                                }
                            } else {
                                eprintln!("No parent folder for file: {path}");
                            }
                        } else {
                            eprintln!("Path not UTF-8: {:?}", entry.path());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading directory: {e}");
                }
            }
        }
    // NTFS filessystem
    // TODO: ok the way we're accessing MFT is so god damn slow (even slower than jwalk!!) w/ the crate being used, we need to switch.
    // TODO: at least, it doesn't use as much memory
    } else {
        #[cfg(target_os = "windows")]
        println!("Using NTFS MFT for path: {}", index_options.path);

        // scan MFT and insert folders/files into the database
        let drive_letter = get_drive_letter(index_options.path.clone());
        let volume = Volume::from_drive_letter(drive_letter).unwrap();
        let mft = Mft::new(volume);
        let mut path_resolver = MftPathResolver::new(&mft);

        let mut folder_stmt = transaction
            .prepare("INSERT OR IGNORE INTO folders (path) VALUES (?1)")
            .unwrap();
        let mut file_stmt = transaction
            .prepare("INSERT OR IGNORE INTO files (path, folder_id) VALUES (?1, ?2)")
            .unwrap();
        let mut folder_map = HashMap::new();

        println!("Starting MFT scan...");
        for entry in mft.iter() {
            // try to find the path if it exists
            match path_resolver.resolve_path(&entry) {
                Some(path_buf) => {
                    // check if needed to be excluded
                    if src_lib::should_exclude(
                        &path_buf,
                        index_options.clone(),
                        &mut exclude_counts,
                    ) {
                        ignored += 1;
                        continue;
                    }

                    let path_str = path_buf.to_str().unwrap_or("<invalid utf8>");
                    // if directory, insert
                    if entry.is_dir() {
                        let rows = folder_stmt.execute(rusqlite::params![path_str]).unwrap();
                        if rows > 0 {
                            folders_found += 1
                        } else {
                            // Already exists in DB
                            exists += 1;
                        }
                        if !folder_map.contains_key(path_str) {
                            let id: i64 = transaction
                                .query_row(
                                    "SELECT id FROM folders WHERE path = ?1",
                                    rusqlite::params![path_str],
                                    |row| row.get(0),
                                )
                                .unwrap();
                            folder_map.insert(path_str.to_string(), id);
                        }
                    // if user wants to index files, insert them too
                    } else if index_options.index_files {
                        if let Some(parent) = std::path::Path::new(path_str)
                            .parent()
                            .and_then(|p| p.to_str())
                        {
                            let folder_id = if let Some(folder_id) = folder_map.get(parent) {
                                *folder_id
                            } else {
                                // find parent folder in DB
                                match transaction.query_row(
                                    "SELECT id FROM folders WHERE path = ?1",
                                    rusqlite::params![parent],
                                    |row| row.get(0),
                                ) {
                                    Ok(folder_id) => {
                                        folder_map.insert(parent.to_string(), folder_id);
                                        folder_id
                                    }
                                    Err(_) => {
                                        // create parent folder in DB if it doesn't exist
                                        let rows =
                                            folder_stmt.execute(rusqlite::params![parent]).unwrap();
                                        if rows > 0 {
                                            folders_found += 1
                                        } else {
                                            // Already exists in DB
                                            exists += 1;
                                        }
                                        let folder_id: i64 = transaction
                                            .query_row(
                                                "SELECT id FROM folders WHERE path = ?1",
                                                rusqlite::params![parent],
                                                |row| row.get(0),
                                            )
                                            .unwrap();
                                        folder_map.insert(parent.to_string(), folder_id);
                                        folder_id
                                    }
                                }
                            };
                            let rows = file_stmt
                                .execute(rusqlite::params![path_str, &folder_id])
                                .unwrap();
                            if rows > 0 {
                                files_found += 1
                            } else {
                                // Already exists in DB
                                exists += 1;
                            }
                        } else {
                            eprintln!("No parent folder for file: {path_str}");
                        }
                    }
                }
                None => {
                    continue;
                }
            }
        }
    }
    let duration = start_time.elapsed();

    transaction
        .commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;
    println!(
        "Indexing completed in {:.3?} ({} folders, {} files, {} ignored, {} already exists)",
        duration, folders_found, files_found, ignored, exists
    );
    println!(
        "Excluded counts: {}",
        exclude_counts
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(format!(
        "Indexed {} new folders and {} new files in {:.3?} ({} ignored, {} already exists)",
        folders_found, files_found, duration, ignored, exists
    ))
}
