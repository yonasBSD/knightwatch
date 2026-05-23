use directories::ProjectDirs;
use std::path::PathBuf;

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", "", "knightwatch").expect("Could not determine app data directory")
}

pub fn file_path(file: &'static str) -> PathBuf {
    project_dirs().config_dir().join(file)
}

pub fn dir_path() -> PathBuf {
    project_dirs().config_dir().to_path_buf()
}
