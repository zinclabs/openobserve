// Copyright 2023 Zinc Labs Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::{
    fs::{File, Metadata},
    io::{Read, Write},
    path::Path,
};

use async_walkdir::WalkDir;
use futures::StreamExt;

#[inline(always)]
pub fn get_file_meta(file: &str) -> Result<Metadata, std::io::Error> {
    let file = File::open(file)?;
    file.metadata()
}

#[inline(always)]
pub fn get_file_contents(file: &str) -> Result<Vec<u8>, std::io::Error> {
    let mut file = File::open(file)?;
    let mut contents: Vec<u8> = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(contents)
}

#[inline(always)]
pub fn put_file_contents(file: &str, contents: &[u8]) -> Result<(), std::io::Error> {
    let mut file = File::create(file)?;
    file.write_all(contents)
}

#[inline(always)]
pub async fn scan_files<P: AsRef<Path>>(root: P, ext: &str) -> Vec<String> {
    let mut wd = WalkDir::new(root);
    let mut resp = Vec::new();
    loop {
        match wd.next().await {
            Some(Ok(entry)) => {
                let path = entry.path();
                if path.is_file() {
                    match path.extension() {
                        Some(e) => {
                            if e == ext {
                                resp.push(path.to_str().unwrap().to_string())
                            }
                        }
                        None => {}
                    }
                } else {
                    continue;
                }
            }
            Some(Err(_)) => {}
            None => break,
        }
    }
    resp
}

pub async fn clean_empty_dirs(dir: &str) -> Result<(), std::io::Error> {
    let mut dirs = Vec::new();
    let mut wd = async_walkdir::WalkDir::new(dir);

    loop {
        match wd.next().await {
            Some(Ok(entry)) => {
                if entry.path().display().to_string() == dir {
                    continue;
                }
                match entry.file_type().await {
                    Ok(ft) => {
                        if ft.is_dir() {
                            dirs.push(entry.path().to_str().unwrap().to_string())
                        };
                    }
                    Err(_) => {}
                }
            }
            Some(Err(_)) => {}
            None => break,
        }
    }

    dirs.sort_by_key(|b| std::cmp::Reverse(b.len()));
    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            if entries.count() == 0 {
                std::fs::remove_dir(&dir)?;
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
pub fn set_permission<P: AsRef<std::path::Path>>(path: P, mode: u32) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path.as_ref())?;
    std::fs::set_permissions(path.as_ref(), std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
pub fn set_permission<P: AsRef<std::path::Path>>(
    path: P,
    _mode: u32,
) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(path.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file() {
        let content = b"Some Text";
        let file_name = "sample.parquet";

        put_file_contents(file_name, content).unwrap();
        assert_eq!(get_file_contents(file_name).unwrap(), content);
        assert!(get_file_meta(file_name).unwrap().is_file());
        assert!(!scan_files(".", "parquet").is_empty());
        std::fs::remove_file(file_name).unwrap();
    }
}
