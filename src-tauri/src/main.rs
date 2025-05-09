// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;

use rusqlite::Connection;
use tauri::AppHandle;

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
fn index_directory(app_handle: AppHandle, path_string: String) -> Result<String, String> {
    let db_path = guessfs_lib::get_index_db_path(&app_handle, &path_string)?;

    let db = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    if !db_path.exists() {
        guessfs_lib::init_db(&db).map_err(|e| format!("Failed to initialize database: {}", e))?;
        println!("New database created at: {}", db_path.display());
    }

    let path = Path::new(&path_string);
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path_string));
    }

    let is_ntfs = guessfs_lib::is_ntfs(&path);
    if !is_ntfs {
        println!("Directory is not on NTFS filesystem: {}", path_string);
    } else {
        println!("Directory is on NTFS filesystem: {}", path_string);
    }

    Ok("Directory indexed successfully".into())
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
