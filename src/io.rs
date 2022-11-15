use std::fs::{
    write,
    read_to_string,
    create_dir_all
};
use std::path::{Path, PathBuf};
use std::io::Result;
use home::home_dir;

pub fn write_to_file(path: &str, content: &str) -> Result<()> {
    write(Path::new(path), content)
}

pub fn read_from_file(path: &str) -> Result<String> {
    read_to_string(Path::new(path))
}

pub fn get_cache_folder(provider: &str) -> Option<PathBuf> {
    let path = home_dir().and_then(|mut p| {
        p.push(".cache/leetcode-cli/");
        p.push(provider);
        Some(p)
    });
    if path.is_some() && create_dir_all(path.clone().unwrap()).is_ok() {
        return path;
    }
    None
}