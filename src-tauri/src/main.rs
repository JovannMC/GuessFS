// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use guessfs_lib::{get_drive_letter, IndexOptions};
use jwalk::WalkDir;
use rusqlite::Connection;
use std::time::Instant;
use tauri::AppHandle;
use usn_journal_rs::{mft::Mft, path_resolver::MftPathResolver};

static STOP_FLAG: AtomicBool = AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            start_indexing,
            stop_indexing,
            get_random_dir,
            get_random_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn start_indexing(app_handle: AppHandle, index_options: IndexOptions) -> Result<String, String> {
    let mut exclude_counts: HashMap<&'static str, usize> = HashMap::new();

    STOP_FLAG.store(false, Ordering::Relaxed);
    tauri::async_runtime::spawn(async move {
        let db_path = guessfs_lib::get_index_db_path(&app_handle, &index_options.path)?;

        let mut db =
            Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

        if !db_path.exists() {
            guessfs_lib::init_db(&db)
                .map_err(|e| format!("Failed to initialize database: {}", e))?;
            println!("New database created at: {}", db_path.display());
        } else {
            guessfs_lib::init_db(&db)
                .map_err(|e| format!("Failed to initialize database: {}", e))?;
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

        // TODO: check how is this handled in linux/macos
        let is_root = path
            .components()
            .count()
            == 1; // check if the path is a root directory (e.g., C:\)

        let is_ntfs = guessfs_lib::is_ntfs(&path);
        // Non-NTFS filesystem / not Windows
        // also check if the path is not a root directory, because with MFT we can only index the entire root
        if !is_ntfs || (is_ntfs && !is_root) {
            println!(
                "Not using NTFS MFT for path: {}",
                index_options.path
            );
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
                // break if user wants to stop indexing
                if STOP_FLAG.load(Ordering::Relaxed) {
                    println!(
                        "Indexing stopped by user in {} seconds for path: {}",
                        start_time.elapsed().as_secs(),
                        index_options.path
                    );
                    println!("Folders found: {}", folders_found);
                    println!("Files found: {}", files_found);
                    println!(
                        "Ignored: {}",
                        exclude_counts.iter().map(|(_, v)| v).sum::<usize>()
                    );
                    return Ok(format!("Indexing stopped for path: {}", index_options.path));
                }

                match entry {
                    Ok(entry) => {
                        // check if needed to be excluded
                        if guessfs_lib::should_exclude(
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
                                if folder_stmt.execute(rusqlite::params![path]).unwrap() > 0 {
                                    folders_found += 1
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
                                        if file_stmt
                                            .execute(rusqlite::params![path, folder_id])
                                            .unwrap()
                                            > 0
                                        {
                                            files_found += 1
                                        }
                                    } else {
                                        // if not in map, fetch from DB
                                        if let Ok(folder_id) = transaction.query_row(
                                            "SELECT id FROM folders WHERE path = ?1",
                                            rusqlite::params![parent],
                                            |row| row.get(0),
                                        ) {
                                            folder_map.insert(parent.to_string(), folder_id);
                                            if file_stmt
                                                .execute(rusqlite::params![path, &folder_id])
                                                .unwrap()
                                                > 0
                                            {
                                                files_found += 1
                                            }
                                        } else {
                                            // not found, create parent folder in DB
                                            if folder_stmt
                                                .execute(rusqlite::params![parent])
                                                .unwrap()
                                                > 0
                                            {
                                                folders_found += 1
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
            let mft = Mft::new_from_drive_letter(drive_letter).unwrap();
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
                // break if user wants to stop indexing
                if STOP_FLAG.load(Ordering::Relaxed) {
                    println!(
                        "Indexing stopped by user in {} seconds for path: {}",
                        start_time.elapsed().as_secs(),
                        index_options.path
                    );
                    println!("Folders found: {}", folders_found);
                    println!("Files found: {}", files_found);
                    println!(
                        "Ignored: {}",
                        exclude_counts.iter().map(|(_, v)| v).sum::<usize>()
                    );
                    return Ok(format!("Indexing stopped for path: {}", index_options.path));
                }

                // try to find the path if it exists
                match path_resolver.resolve_path(&entry) {
                    Some(path_buf) => {
                        // check if needed to be excluded
                        if guessfs_lib::should_exclude(
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
                            if folder_stmt.execute(rusqlite::params![path_str]).unwrap() > 0 {
                                folders_found += 1
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
                                            if folder_stmt
                                                .execute(rusqlite::params![parent])
                                                .unwrap()
                                                > 0
                                            {
                                                folders_found += 1
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
                                if file_stmt
                                    .execute(rusqlite::params![path_str, &folder_id])
                                    .unwrap()
                                    > 0
                                {
                                    files_found += 1
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
            "Indexing completed in {:.3?} ({} folders, {} files, {} ignored)",
            duration, folders_found, files_found, ignored
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
            "Indexed {} folders and {} files in {:.2?} ({} ignored)",
            folders_found, files_found, duration, ignored
        ))
    });
    Ok("Indexing started".to_string())
}

#[tauri::command]
fn stop_indexing() {
    // stop indexing
    STOP_FLAG.store(true, Ordering::Relaxed);
    println!("Indexing stopped");
}

#[tauri::command]
fn get_random_dir(app_handle: AppHandle, path_string: String) -> Result<String, String> {
    let db_path = guessfs_lib::get_index_db_path(&app_handle, &path_string).unwrap();
    let db = Connection::open(&db_path).unwrap();

    // prepare and execute the SQL statement
    let mut stmt = db
        .prepare("SELECT path FROM folders ORDER BY RANDOM() LIMIT 1")
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("Failed to execute query: {}", e))?;
    if let Some(row) = rows
        .next()
        .map_err(|e| format!("Failed to get next row: {}", e))?
    {
        let path: String = row
            .get(0)
            .map_err(|e| format!("Failed to get path from row: {}", e))?;
        return Ok(path);
    }
    Err("No files found in DB".to_string())
}

#[tauri::command]
fn get_random_file(app_handle: AppHandle, path_string: String) -> Result<String, String> {
    let db_path = guessfs_lib::get_index_db_path(&app_handle, &path_string).unwrap();
    let db = Connection::open(&db_path).unwrap();

    // prepare and execute the SQL statement
    let mut stmt = db
        .prepare("SELECT path FROM files ORDER BY RANDOM() LIMIT 1")
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("Failed to execute query: {}", e))?;
    if let Some(row) = rows
        .next()
        .map_err(|e| format!("Failed to get next row: {}", e))?
    {
        let path: String = row
            .get(0)
            .map_err(|e| format!("Failed to get path from row: {}", e))?;
        return Ok(path);
    }
    Err("No files found in DB".to_string())
}
