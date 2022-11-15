use std::fs::{
    write,
    read_to_string,
    create_dir_all
};
use std::convert::AsRef;
use std::path::{Path, PathBuf};
use std::io::Result;
use home::home_dir;

pub fn write_to_file<T: AsRef<Path>>(path: T, content: &str) -> Result<()> {
    write(path, content)
}

pub fn read_from_file<T: AsRef<Path>>(path: T) -> Result<String> {
    read_to_string(path)
}

pub fn get_cache_folder(provider: &str) -> Option<PathBuf> {
    home_dir().and_then(|mut p| {
        p.push(".cache/leetcode-cli/");
        p.push(provider);
        create_dir_all(&p).ok()?;
        Some(p)
    })
}