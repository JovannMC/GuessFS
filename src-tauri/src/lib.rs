use rusqlite::{Connection, Result as RusqliteResult};
use sha2::{Digest, Sha256};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::{fs::create_dir_all, path::PathBuf};
use tauri::{AppHandle, Manager};
use winapi::um::fileapi::GetVolumeInformationW;

pub fn get_index_db_path(
    app_handle: &AppHandle,
    directory_path_str: &str,
) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    if !app_data_dir.exists() {
        create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    }

    let mut hasher = Sha256::new();
    hasher.update(directory_path_str.as_bytes());
    let hash_result = hasher.finalize();
    let db_file_name = format!("index_{:x}.db", hash_result);

    Ok(app_data_dir.join(db_file_name))
}

pub fn init_db(conn: &Connection) -> RusqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS folders (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE NOT NULL,
            folder_id INTEGER NOT NULL,
            FOREIGN KEY (folder_id) REFERENCES folders(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_folder_path ON folders (path);",
        [],
    )?;
    Ok(())
}

pub fn get_drive_letter(path_string: String) -> char {
    path_string
        .chars()
        .take_while(|c| *c != ':')
        .next()
        .expect("No drive letter found in path string")
}

pub fn is_ntfs(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        let root_path = path
            .components()
            .next()
            .map(|c| c.as_os_str().to_os_string())
            .unwrap_or_default();
        if root_path.is_empty() {
            // no drive letter found
            return false;
        }
        let mut root_path_w: Vec<u16> = root_path.encode_wide().collect();
        if !root_path_w.ends_with(&[b'\\' as u16]) {
            root_path_w.push(b'\\' as u16);
        }
        root_path_w.push(0);

        let mut fs_name_buf = [0u16; 32];
        let res = unsafe {
            GetVolumeInformationW(
                root_path_w.as_ptr(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                fs_name_buf.as_mut_ptr(),
                fs_name_buf.len() as u32,
            )
        };
        if res == 0 {
            println!("Failed to get volume information for path: {:?}", path);
            return false;
        }
        let fs_name = String::from_utf16_lossy(&fs_name_buf);
        fs_name
            .trim_matches(char::from(0))
            .eq_ignore_ascii_case("NTFS")
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
