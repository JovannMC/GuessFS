use regex::Regex;
use rusqlite::{Connection, Result as RusqliteResult};
use sha2::{Digest, Sha256};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::{fs::create_dir_all, path::PathBuf};
use std::collections::HashMap;
use tauri::{AppHandle, Manager};
use winapi::um::fileapi::GetVolumeInformationW;
use winapi::um::winnt;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexOptions {
    pub path: String,

    pub index_directories: bool,
    pub index_files: bool,
    pub file_types: Option<Vec<String>>,

    // manual exclusion list set by user
    pub excluded_regex: Option<String>,
    pub excluded_paths: Option<Vec<String>>,
    pub excluded_files: Option<Vec<String>>,

    // friendly exclusion list set by user
    pub exclude_hidden: Option<bool>, // hidden files and directories
    pub exclude_system: Option<bool>, // e.g. $Recycle.Bin, C:\ProgramData, C:\Windows, etc.
    pub exclude_temporary: Option<bool>, // e.g. %TEMP%, etc.
    pub exclude_empty: Option<bool>,  // empty files/folders
    pub exclude_admin: Option<bool>,  // files not accessible by the current user
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

pub fn should_exclude(
    path: &Path,
    options: IndexOptions,
    exclude_counts: &mut HashMap<&'static str, usize>,
) -> bool {
    // windows specific checks
    #[cfg(target_os = "windows")]
    {
        use std::fs::File;
        use std::os::windows::fs::MetadataExt;

        // check if the user can access the file/folder
        if options.exclude_admin == Some(true) {
            let can_access = if path.is_dir() {
                path.read_dir().is_ok()
            } else {
                File::open(path).is_ok()
            };

            if !can_access {
                *exclude_counts.entry("exclude_admin").or_insert(0) += 1;
                return true;
            }
        }

        // check if folder should be excluded
        if let Some(ref excluded_paths) = options.excluded_paths {
            if let Some(path_str) = path.to_str() {
                for exclude_path in excluded_paths {
                    if path_str.starts_with(exclude_path) {
                        *exclude_counts.entry("excluded_paths").or_insert(0) += 1;
                        return true;
                    }
                }
            }
        }

        // check if the file should be excluded
        if let Some(ref excluded_files) = options.excluded_files {
            if let Some(path_str) = path.to_str() {
                for exclude_file in excluded_files {
                    if path_str.ends_with(exclude_file) {
                        *exclude_counts.entry("excluded_files").or_insert(0) += 1;
                        return true;
                    }
                }
            }
        }

        // exclude regex
        if let Some(ref excluded_regex) = options.excluded_regex {
            let re = Regex::new(excluded_regex).unwrap();
            if re.is_match(path.to_str().unwrap_or("")) {
                *exclude_counts.entry("excluded_regex").or_insert(0) += 1;
                return true;
            }
        }

        if let Ok(metadata) = path.metadata() {
            let attributes = metadata.file_attributes();

            // hidden files and folders
            if options.exclude_hidden == Some(true)
                && (attributes & winnt::FILE_ATTRIBUTE_HIDDEN) != 0
            {
                *exclude_counts.entry("exclude_hidden").or_insert(0) += 1;
                return true;
            }

            // exclude protected system files
            if options.exclude_system == Some(true)
                && (attributes & winnt::FILE_ATTRIBUTE_SYSTEM) != 0
            {
                *exclude_counts.entry("exclude_system").or_insert(0) += 1;
                return true;
            }

            // exclude empty folders and files
            if options.exclude_empty == Some(true) {
                if path.is_dir() {
                    if let Ok(entries) = path.read_dir() {
                        if entries.count() == 0 {
                            *exclude_counts.entry("exclude_empty").or_insert(0) += 1;
                            return true;
                        }
                    }
                } else if path.is_file() {
                    if metadata.len() == 0 {
                        *exclude_counts.entry("exclude_empty").or_insert(0) += 1;
                        return true;
                    }
                }
            }

            // exclude temporary files and folders
            if options.exclude_temporary == Some(true) {
                if (attributes & winnt::FILE_ATTRIBUTE_TEMPORARY) != 0 {
                    *exclude_counts.entry("exclude_temporary").or_insert(0) += 1;
                    return true;
                }
                // check for common temp folder/file names
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let lower = name.to_ascii_lowercase();
                    if lower == "temp" || lower == "tmp" || lower.ends_with(".tmp") {
                        *exclude_counts.entry("exclude_temporary").or_insert(0) += 1;
                        return true;
                    }
                }
            }
            // TODO: more checks maybe?
        }
        false
    }

    // linux/macOS specific checks
    #[cfg(not(target_os = "windows"))]
    {
        let path = Path::new(&path);
        if options.exclude_hidden == Some(true) {
            if let Some(name) = path.file_name() {
                if name.to_str().map_or(false, |s| s.starts_with('.')) {
                    *exclude_counts.entry("exclude_hidden").or_insert(0) += 1;
                    return true;
                }
            }
        }
        false
    }
}
