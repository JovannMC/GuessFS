// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use guessfs_lib::IndexOptions;
use rusqlite::Connection;
use tauri::{AppHandle, Manager};

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
    // start indexing
    let app_data_dir = app_handle.path().app_data_dir().unwrap();
    Ok("Indexing started".to_string())
}

#[tauri::command]
fn stop_indexing() {
    // stop indexing
    println!("Indexing stopped");
}

#[tauri::command]
fn get_random_dir(app_handle: AppHandle, path_string: String) -> Result<String, String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap();
    let db_path = guessfs_lib::get_index_db_path(&app_data_dir, &path_string).unwrap();
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
    let app_data_dir = app_handle.path().app_data_dir().unwrap();
    let db_path = guessfs_lib::get_index_db_path(&app_data_dir, &path_string).unwrap();
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
