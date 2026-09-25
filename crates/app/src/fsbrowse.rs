// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Server-side file browsing for the script picker. Lists names and metadata only, never file
//! contents. Callers must have checked that the user is an administrator.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use kaaryasoochi_core::dto::{DirListing, FileInfo, FsEntry};

/// Cap on returned entries so a huge directory cannot bloat the response.
const MAX_ENTRIES: usize = 2000;

fn is_exec(meta: &fs::Metadata) -> bool {
    meta.is_file() && meta.permissions().mode() & 0o111 != 0
}

/// Lists `path` (empty means the user's home). `path` is canonicalised, so symlinks resolve.
pub fn list_dir(path: &str, show_hidden: bool) -> Result<DirListing, String> {
    let requested = if path.trim().is_empty() {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
    } else {
        PathBuf::from(path.trim())
    };
    if !requested.is_absolute() {
        return Err("path must be absolute".into());
    }
    let dir = requested
        .canonicalize()
        .map_err(|e| format!("cannot open {}: {e}", requested.display()))?;
    if !dir.is_dir() {
        return Err(format!("{} is not a directory", dir.display()));
    }
    let read = fs::read_dir(&dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;

    let mut entries: Vec<FsEntry> = read
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if !show_hidden && name.starts_with('.') {
                return None;
            }
            // Follow symlinks so a link to a directory browses like one; skip dangling links.
            let meta = fs::metadata(e.path()).ok()?;
            Some(FsEntry {
                name,
                is_dir: meta.is_dir(),
                is_executable: is_exec(&meta),
                size: if meta.is_file() { meta.len() } else { 0 },
            })
        })
        .collect();
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    let truncated = entries.len() > MAX_ENTRIES;
    entries.truncate(MAX_ENTRIES);

    Ok(DirListing {
        path: dir.to_string_lossy().into_owned(),
        parent: dir.parent().map(|p| p.to_string_lossy().into_owned()),
        entries,
        truncated,
    })
}

pub fn file_info(path: &str) -> FileInfo {
    match fs::metadata(Path::new(path.trim())) {
        Ok(m) => FileInfo {
            exists: true,
            is_file: m.is_file(),
            is_dir: m.is_dir(),
            is_executable: is_exec(&m),
        },
        Err(_) => FileInfo {
            exists: false,
            is_file: false,
            is_dir: false,
            is_executable: false,
        },
    }
}

/// Adds the owner execute bit to a regular file. Refuses anything else.
pub fn make_executable(path: &str) -> Result<FileInfo, String> {
    let p = Path::new(path.trim());
    let meta = fs::metadata(p).map_err(|e| format!("cannot access {}: {e}", p.display()))?;
    if !meta.is_file() {
        return Err(format!("{} is not a regular file", p.display()));
    }
    let mut perms = meta.permissions();
    perms.set_mode(perms.mode() | 0o100);
    fs::set_permissions(p, perms).map_err(|e| format!("cannot change permissions: {e}"))?;
    Ok(file_info(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "kaarya-fs-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("sub")).unwrap();
        fs::write(d.join("b.sh"), "#!/bin/sh\n").unwrap();
        fs::write(d.join("A.txt"), "x").unwrap();
        fs::write(d.join(".hidden"), "x").unwrap();
        fs::set_permissions(d.join("b.sh"), fs::Permissions::from_mode(0o755)).unwrap();
        d
    }

    #[test]
    fn lists_dirs_first_then_names_case_insensitively() {
        let d = scratch();
        let l = list_dir(d.to_str().unwrap(), false).unwrap();
        let names: Vec<_> = l.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            ["sub", "A.txt", "b.sh"],
            "dirs first; hidden files skipped"
        );
        assert!(
            l.entries
                .iter()
                .find(|e| e.name == "b.sh")
                .unwrap()
                .is_executable
        );
        assert!(
            !l.entries
                .iter()
                .find(|e| e.name == "A.txt")
                .unwrap()
                .is_executable
        );
        assert!(list_dir(d.to_str().unwrap(), true)
            .unwrap()
            .entries
            .iter()
            .any(|e| e.name == ".hidden"));
        assert_eq!(l.parent.as_deref(), d.parent().and_then(|p| p.to_str()));
        assert!(!l.truncated);
    }

    #[test]
    fn rejects_bad_paths() {
        assert!(list_dir("relative/dir", false).is_err());
        assert!(list_dir("/definitely/not/here", false).is_err());
        let d = scratch();
        assert!(
            list_dir(d.join("A.txt").to_str().unwrap(), false).is_err(),
            "a file is not a directory"
        );
        assert!(list_dir("", false).is_ok(), "empty means home");
    }

    #[test]
    fn inspects_and_chmods_files_only() {
        let d = scratch();
        let f = d.join("A.txt");
        let before = file_info(f.to_str().unwrap());
        assert!(before.exists && before.is_file && !before.is_executable);
        assert!(make_executable(f.to_str().unwrap()).unwrap().is_executable);
        assert!(make_executable(d.join("sub").to_str().unwrap()).is_err());
        assert!(!file_info(d.join("nope").to_str().unwrap()).exists);
    }

    #[test]
    fn follows_symlinked_directories_and_skips_dangling_links() {
        let d = scratch();
        std::os::unix::fs::symlink(d.join("sub"), d.join("link")).unwrap();
        std::os::unix::fs::symlink(d.join("missing"), d.join("dangling")).unwrap();
        let l = list_dir(d.to_str().unwrap(), false).unwrap();
        assert!(l.entries.iter().find(|e| e.name == "link").unwrap().is_dir);
        assert!(l.entries.iter().all(|e| e.name != "dangling"));
    }
}
