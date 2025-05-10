// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;

use jwalk::WalkDir;
use rusqlite::Connection;
use std::time::Instant;
use tauri::AppHandle;

mod mft;
use crate::mft::mft::iter_mft;

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

    let start_time = Instant::now();

    let is_ntfs = guessfs_lib::is_ntfs(&path);
    let mut found_dirs = Vec::new();
    let mut found_files = Vec::new();

    let find_start = Instant::now();
    if !is_ntfs {
        println!("Directory is not on NTFS filesystem: {}", path_string);
        for entry in WalkDir::new(path) {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_dir() {
                        if let Some(path) = entry.path().to_str() {
                            found_dirs.push(path.to_string());
                        } else {
                            eprintln!("Path not UTF-8: {:?}", entry.path());
                        }
                    } else if index_files {
                        if let Some(path) = entry.path().to_str() {
                            found_files.push(path.to_string());
                        } else {
                            eprintln!("Path not UTF-8: {:?}", entry.path());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading directory: {}", e);
                }
            }
        }
    } else {
        println!("Directory is on NTFS filesystem: {}", path_string);
        let (dirs, files) = iter_mft(path_string.clone())
            .map_err(|e| format!("Failed to index NTFS volume: {}", e))?;
        found_dirs = dirs;
        found_files = files;
    }
    let find_duration = find_start.elapsed();

    let push_start = Instant::now();
    for dir_path in &found_dirs {
        if guessfs_lib::push_folder(&transaction, dir_path) {
            folders_found += 1
        }
    }
    if index_files {
        for file_path in &found_files {
            if guessfs_lib::push_file(&transaction, file_path) {
                files_found += 1
            }
        }
    }
    let push_duration = push_start.elapsed();

    let duration = start_time.elapsed();

    println!("Time to find entries: {:.2?}", find_duration);
    println!("Time to push to DB: {:.2?}", push_duration);

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
