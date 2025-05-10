#[cfg(target_os = "windows")]
pub mod mft {
    use std::time::Instant;
    use usn_journal_rs::mft::Mft;
    use usn_journal_rs::path_resolver::MftPathResolver;

    fn get_drive_letter(path_string: String) -> char {
        path_string
            .chars()
            .take_while(|c| *c != ':')
            .next()
            .expect("No drive letter found in path string")
    }

    pub fn iter_mft(path_string: String) -> Result<(Vec<String>, Vec<String>), String> {
        let drive_letter = get_drive_letter(path_string.clone());
        let mft = Mft::new_from_drive_letter(drive_letter).unwrap();
        let mut path_resolver = MftPathResolver::new(&mft);

        let mut found_dirs: Vec<String> = Vec::new();
        let mut found_files: Vec<String> = Vec::new();
        println!("Starting MFT scan...");

        let mut file_count = 0;
        let mut dir_count = 0;

        let start_time = Instant::now();

        for entry in mft.iter() {
            match path_resolver.resolve_path(&entry) {
                Some(path_buf) => {
                    found_dirs.push(path_buf.to_str().unwrap_or("<invalid utf8>").to_string());
                    if entry.is_dir() {
                        dir_count += 1;
                        found_dirs.push(path_buf.to_str().unwrap_or("<invalid utf8>").to_string());
                    } else {
                        file_count += 1;
                        found_files.push(path_buf.to_str().unwrap_or("<invalid utf8>").to_string());
                    }
                }
                None => {
                    println!("Could not resolve path for entry: {:?}", entry);
                    continue;
                }
            }
        }

        let duration = start_time.elapsed();
        println!(
            "MFT scan completed in {}.{:03} seconds",
            duration.as_secs(),
            duration.subsec_millis()
        );

        println!("Found {} files and {} directories", file_count, dir_count);

        Ok((found_dirs, found_files))
    }
}
