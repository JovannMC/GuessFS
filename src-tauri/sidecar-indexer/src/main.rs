use clap::{Command, Arg, value_parser, ArgAction};

fn main() {
    let matches = Command::new("Indexer")
        .version("1.0")
        .arg(Arg::new("path").long("path").value_parser(value_parser!(String)).help("Path to index").required(true))
        .arg(Arg::new("index").long("index").value_parser(value_parser!(String)).help("Index files and/or directories (comma-separated) - files/dirs)").required(true))
        .arg(Arg::new("types").long("types").value_parser(value_parser!(String)).help("File types to index (comma-separated)"))
        .arg(Arg::new("exclude").long("exclude").value_parser(value_parser!(String)).help("Exclude common unwanted files (comma-separated) - empty, temp, hidden, system, privileged"))
        .arg(Arg::new("exclude_regex").long("exclude-regex").value_parser(value_parser!(String)).help("Exclude files or directories matching this regex pattern"))
        .arg(Arg::new("exclude_paths").long("exclude-paths").value_parser(value_parser!(String)).help("Exclude specific paths (comma-separated)"))
        .arg(Arg::new("exclude_files").long("exclude-files").value_parser(value_parser!(String)).help("Exclude specific files (comma-separated)"))
        .arg(Arg::new("elevate").long("elevate").help("Request elevation").action(ArgAction::SetTrue))
        .get_matches();

    if matches.get_flag("elevate") {
        // TODO: handle elevation request
        #[cfg(target_os = "windows")]
        {
            println!("meow ur on windows");
            println!("path: {}", matches.get_one::<String>("path").unwrap());
            println!("indexing: {}", matches.get_one::<String>("index").unwrap());
        }
        #[cfg(target_os = "linux")]
        {
            println!("meow ur on linux");
            println!("path: {}", matches.get_one::<String>("path").unwrap());
            println!("indexing: {}", matches.get_one::<String>("index").unwrap());
        }
        #[cfg(target_os = "macos")]
        {
            println!("meow ur on macos");
            println!("path: {}", matches.get_one::<String>("path").unwrap());
            println!("indexing: {}", matches.get_one::<String>("index").unwrap());
        }
    }
}
