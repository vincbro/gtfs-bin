use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use zip::ZipArchive;

use crate::{Error, Result};

/// Where the feed's text files are read from.
pub(super) enum Source {
    Directory(PathBuf),
    Zip(ZipArchive<File>),
}

impl Source {
    /// Opens a directory of text files, or a zip archive of them.
    pub(super) fn open(path: &Path) -> Result<Self> {
        if path.is_dir() {
            return Ok(Self::Directory(path.to_path_buf()));
        }
        let file = File::open(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self::Zip(ZipArchive::new(file)?))
    }

    /// Opens the named file, or returns `None` if the feed doesn't have it.
    ///
    /// In an archive the file may sit in a folder, because some publishers zip the folder itself.
    pub(super) fn read(&mut self, name: &str) -> Result<Option<Box<dyn Read + '_>>> {
        match self {
            Self::Directory(directory) => {
                let path = directory.join(name);
                if !path.is_file() {
                    return Ok(None);
                }
                let file = File::open(&path).map_err(|source| Error::Io { path, source })?;
                Ok(Some(Box::new(file)))
            }
            Self::Zip(archive) => {
                let Some(index) = find_entry(archive, name) else {
                    return Ok(None);
                };
                Ok(Some(Box::new(archive.by_index(index)?)))
            }
        }
    }
}

fn find_entry(archive: &ZipArchive<File>, name: &str) -> Option<usize> {
    let suffix = format!("/{name}");
    archive
        .file_names()
        .position(|entry| entry.is_ok_and(|entry| entry == name || entry.ends_with(&suffix)))
}
