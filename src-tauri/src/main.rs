// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;

use jwalk::WalkDir;
use rusqlite::Connection;
use tauri::AppHandle;
use std::time::Instant;

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
    let mut folders_found = 0;
    let mut files_found = 0;

    let is_ntfs = guessfs_lib::is_ntfs(&path);
    if !is_ntfs {
        println!("Directory is not on NTFS filesystem: {}", path_string);
    } else {
        println!("Directory is on NTFS filesystem: {}", path_string);
        // TODO: use MFT to index dirs
    }

    let start_time = Instant::now();

    for entry in WalkDir::new(path) {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_dir() {
                    if let Some(path) = entry.path().to_str() {
                        let path = path.to_string();
                        match transaction.execute(
                            "INSERT OR IGNORE INTO folders (path) VALUES (?1)",
                            rusqlite::params![path],
                        ) {
                            Ok(_) => {
                                folders_found += 1;
                            }
                            Err(e) => {
                                eprintln!("Error inserting {} into db: {}", path, e);
                            }
                        }
                    } else {
                        eprintln!("Path not UTF-8: {:?}", entry.path());
                    }
                } else {
                    if index_files {
                        if let Some(path) = entry.path().to_str() {
                            let path = path.to_string();
                            // get parent folder path
                            if let Some(parent) = entry.path().parent() {
                                if let Some(parent_str) = parent.to_str() {
                                    // query for folder_id
                                    match transaction.query_row(
                                        "SELECT id FROM folders WHERE path = ?1",
                                        rusqlite::params![parent_str],
                                        |row| row.get::<_, i64>(0),
                                    ) {
                                        Ok(folder_id) => {
                                            match transaction.execute(
                                                "INSERT OR IGNORE INTO files (path, folder_id) VALUES (?1, ?2)",
                                                rusqlite::params![path, folder_id],
                                            ) {
                                                Ok(_) => files_found += 1,
                                                Err(e) => eprintln!("Error inserting {} into db: {}", path, e),
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("Parent folder not found in db for file {}: {}", path, e);
                                        }
                                    }
                                } else {
                                    eprintln!("Parent path not UTF-8: {:?}", parent);
                                }
                            } else {
                                eprintln!("No parent folder for file: {}", path);
                            }
                        } else {
                            eprintln!("Path not UTF-8: {:?}", entry.path());
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading directory: {}", e);
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
        "Indexing completed in {:.2?} ({} folders, {} files)",
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
