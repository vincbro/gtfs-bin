//! The system's file dialog, and opening the file it picks in a window of its own.

use std::{
    env, io,
    path::{Path, PathBuf},
    process::Command,
};

use rfd::FileDialog;

/// Asks for a compiled file with the system's dialog, which is Finder's on macOS.
///
/// Returns `None` if the dialog was cancelled.
pub(crate) fn pick() -> Option<PathBuf> {
    let mut dialog = FileDialog::new()
        .set_title("Open a compiled flatnet file")
        .add_filter("flatnet", &["flatnet"]);
    if let Ok(directory) = env::current_dir() {
        dialog = dialog.set_directory(directory);
    }
    dialog.pick_file()
}

/// Starts another viewer on `path`. A window holds the one file it was opened with, so a second
/// file gets a second process.
pub(crate) fn open_in_new_window(path: &Path) -> io::Result<()> {
    Command::new(env::current_exe()?).arg(path).spawn()?;
    Ok(())
}
