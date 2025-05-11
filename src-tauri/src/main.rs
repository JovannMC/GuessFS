// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashMap, path::Path};

use guessfs_lib::get_drive_letter;
use jwalk::WalkDir;
use rusqlite::Connection;
use std::time::Instant;
use tauri::AppHandle;
use usn_journal_rs::{mft::Mft, path_resolver::MftPathResolver};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            index_directory,
            get_random_dir,
            get_random_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn index_directory(
    app_handle: AppHandle,
    path_string: String,
    index_files: bool,
) -> Result<String, String> {
    let db_path = guessfs_lib::get_index_db_path(&app_handle, &path_string)?;

    let mut db =
        Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    if !db_path.exists() {
        guessfs_lib::init_db(&db).map_err(|e| format!("Failed to initialize database: {}", e))?;
        println!("New database created at: {}", db_path.display());
    } else {
        guessfs_lib::init_db(&db).map_err(|e| format!("Failed to initialize database: {}", e))?;
        println!("Database already exists at: {}", db_path.display());
    }

    let path = Path::new(&path_string);
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path_string));
    }

    let transaction = db
        .transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    let start_time = Instant::now();
    let mut folders_found = 0;
    let mut files_found = 0;

    let is_ntfs = guessfs_lib::is_ntfs(&path);
    // Non-NTFS filesystem / not Windows
    if !is_ntfs {
        println!("Directory is not on NTFS filesystem: {path_string}");
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
                    } else if index_files {
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
                                        eprintln!("Parent folder not found in db for file {path}");
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
    } else {
        #[cfg(target_os = "windows")]
        println!("Directory is on NTFS filesystem: {path_string}");

        // scan MFT and insert folders/files into the database
        let drive_letter = get_drive_letter(path_string.clone());
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
            // try to find the path if it exists
            match path_resolver.resolve_path(&entry) {
                Some(path_buf) => {
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
                    } else if index_files {
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
                                        if folder_stmt.execute(rusqlite::params![parent]).unwrap() > 0 {
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
    println!("Indexed {} folders", folders_found);
    if index_files {
        println!("Indexed {} files", files_found);
    }
    println!(
        "Indexing completed in {:.3?} ({} folders, {} files)",
        duration, folders_found, files_found
    );

    Ok(format!(
        "Indexed {} folders and {} files in {:.2?}",
        folders_found, files_found, duration
    ))
}

#[tauri::command]
fn get_random_dir(app_handle: AppHandle) -> String {
    // query database
    "meow".to_string()
}

#[tauri::command]
fn get_random_file(app_handle: AppHandle) -> String {
    // query database
    "meow".to_string()
}
