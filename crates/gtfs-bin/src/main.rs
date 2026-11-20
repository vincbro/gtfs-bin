//! The `gtfs-bin` command: compiles a GTFS feed into a file `Consumer` can map.

use std::{fs, path::PathBuf, time::Instant};

use anyhow::Context;
use clap::Parser;
use gtfs_bin_compiler::Compiler;
use spinners::{Spinner, Spinners};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long, default_value = "output.gtfs")]
    output: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let started = Instant::now();
    let mut spinner = Spinner::new(Spinners::Dots, "Compiling data".to_owned());
    let bytes = Compiler::new(&args.input)
        .compile()
        .with_context(|| format!("failed to compile {}", args.input.display()))?;
    spinner.stop_with_message(format!(
        "Loading and compiling gtfs took {:?}",
        started.elapsed()
    ));

    let started = Instant::now();
    let mut spinner = Spinner::new(Spinners::Dots, "Writing output to disk".to_owned());
    fs::write(&args.output, &bytes)
        .with_context(|| format!("failed to write {}", args.output.display()))?;
    spinner.stop_with_message(format!(
        "Writing binary to file took {:?}",
        started.elapsed()
    ));
    Ok(())
}
