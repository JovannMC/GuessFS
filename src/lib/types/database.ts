export interface IndexOptions {
    path: String;

    index_directories: boolean;
    index_files: boolean;
    file_types?: string[];

    // custom exclusion list set by user
    excluded_regex?: String;
    excluded_paths?: String[];
    excluded_files?: String[];

    // friendly exclusion list set by user
    exclude_hidden?: boolean; // hidden files and directories
    exclude_system?: boolean; // C:\$Recycle.Bin, C:\ProgramData, C:\Windows, etc.
    exclude_temporary?: boolean; // TEMP, %TEMP%, etc.
    exclude_empty?: boolean; // empty files/folders
    exclude_admin?: boolean; // files not accessible by the current user
}