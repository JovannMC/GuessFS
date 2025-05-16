// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rusqlite::Connection;
use src_lib::IndexOptions;
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::{process::CommandEvent, ShellExt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
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
    let mut args = vec!["--path".to_string(), index_options.path.clone()];

    // Combine index_files and index_directories into a single --index arg, comma-separated if both
    let mut index_values = Vec::new();
    if index_options.index_files {
        index_values.push("files");
    }
    if index_options.index_directories {
        index_values.push("dirs");
    }
    if !index_values.is_empty() {
        push_arg(&mut args, "--index", Some(index_values.join(",")));
    }

    push_arg(
        &mut args,
        "--types",
        index_options.file_types.as_ref().map(|v| v.join(",")),
    );
    push_arg(
        &mut args,
        "--exclude-regex",
        index_options.excluded_regex.as_ref(),
    );
    push_arg(
        &mut args,
        "--exclude-paths",
        index_options.excluded_paths.as_ref().map(|v| v.join(",")),
    );
    push_arg(
        &mut args,
        "--exclude-files",
        index_options.excluded_files.as_ref().map(|v| v.join(",")),
    );
    let mut exclude_values = Vec::new();
    if index_options.exclude_hidden.unwrap_or(false) {
        exclude_values.push("hidden");
    }
    if index_options.exclude_system.unwrap_or(false) {
        exclude_values.push("system");
    }
    if index_options.exclude_temporary.unwrap_or(false) {
        exclude_values.push("temp");
    }
    if index_options.exclude_empty.unwrap_or(false) {
        exclude_values.push("empty");
    }
    if index_options.exclude_admin.unwrap_or(false) {
        exclude_values.push("privileged");
    }
    if !exclude_values.is_empty() {
        push_arg(&mut args, "--exclude", Some(exclude_values.join(",")));
    }

    // run sidecar binary
    let sidecar_command = app_handle.shell().sidecar("src-sidecar").unwrap();
    let (mut rx, mut _child) = sidecar_command
        .args(&args)
        .spawn()
        .expect("Failed to spawn sidecar");

    tauri::async_runtime::spawn(async move {
        // read events such as stdout
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line_bytes) => {
                    let mut line = String::from_utf8_lossy(&line_bytes).to_string();
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    println!("Sidecar stdout: {line}");
                    // Optionally, handle the output here
                }
                CommandEvent::Stderr(line_bytes) => {
                    let mut line = String::from_utf8_lossy(&line_bytes).to_string();
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    println!("Sidecar stderr: {line}");
                }
                CommandEvent::Error(error) => {
                    println!("Sidecar error: {error:?}");
                }
                CommandEvent::Terminated(exit_status) => {
                    println!("Sidecar terminated with status: {exit_status:?}");
                    break;
                }
                _ => {}
            }
        }
    });

    println!("Starting indexing with args: {:?}", &args);

    Ok("Indexing started".to_string())
}

fn push_arg(args: &mut Vec<String>, flag: &str, value: Option<impl ToString>) {
    if let Some(val) = value {
        args.push(flag.to_string());
        args.push(val.to_string());
    }
}

#[tauri::command]
fn stop_indexing() {
    // stop indexing
    println!("Indexing stopped");
}

#[tauri::command]
fn get_random_dir(app_handle: AppHandle, path_string: String) -> Result<String, String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap();
    let db_path = src_lib::get_index_db_path(&app_data_dir, &path_string).unwrap();
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
    let db_path = src_lib::get_index_db_path(&app_data_dir, &path_string).unwrap();
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
