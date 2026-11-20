//! Opens a compiled file in a window: its header, its tables and its stops and shapes on a map.
//!
//! Run with `just view network.flatnet`, or `just view` to pick the file in the system's dialog.

use std::{env, fs::File, path::PathBuf};

use anyhow::{Context, anyhow};
use eframe::NativeOptions;
use flatnet_core::consumer::Consumer;
use memmap2::Mmap;

use crate::{app::Viewer, file_info::FileInfo};

mod app;
mod details;
mod file_info;
mod labels;
mod list;
mod map;
mod overlay;
mod picker;
mod selection;
mod shapes;

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn main() -> anyhow::Result<()> {
    // With no file named, ask for one, and quit quietly if there is none.
    let Some(path) = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(picker::pick)
    else {
        return Ok(());
    };
    let path = path.display().to_string();
    let file = File::open(&path).with_context(|| format!("failed to open {path}"))?;
    // SAFETY: the file is only read, and nothing here modifies it while it is mapped.
    let mmap = unsafe { Mmap::map(&file) }.with_context(|| format!("failed to map {path}"))?;
    let consumer = Consumer::new(&mmap).with_context(|| format!("{path} is not a valid file"))?;
    let info =
        FileInfo::read(&mmap, path.clone()).with_context(|| format!("{path} has no header"))?;

    eframe::run_native(
        "flatnet viewer",
        NativeOptions::default(),
        Box::new(move |context| Ok(Box::new(Viewer::new(consumer, info, &context.egui_ctx)))),
    )
    .map_err(|err| anyhow!("the window failed: {err}"))
}
