//! The `flatnet` command: compiles a GTFS feed into a file `Consumer` can map.

use std::{
    fs,
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use clap::Parser;
use flatnet_compiler::Compiler;
use spinners::{Spinner, Spinners};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long, default_value = "output.flatnet")]
    output: PathBuf,

    /// Adds a key and a text value to the file's JSON metadata, as `key=value`. Repeat it for more.
    #[arg(long, value_name = "KEY=VALUE", value_parser = parse_entry)]
    meta: Vec<(String, String)>,

    /// A file with a JSON object to put in the file's metadata. `--meta` replaces its keys.
    #[arg(long, value_name = "FILE")]
    meta_json: Option<PathBuf>,
}

fn parse_entry(text: &str) -> Result<(String, String), String> {
    text.split_once('=')
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .ok_or_else(|| format!("expected key=value, got {text:?}"))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let started = Instant::now();
    let mut spinner = Spinner::new(Spinners::Dots, "Compiling data".to_owned());
    let mut compiler = Compiler::new(&args.input).compiled_at(unix_seconds());
    if let Some(name) = args.input.file_name() {
        compiler = compiler.source(name.to_string_lossy().into_owned());
    }
    if let Some(path) = &args.meta_json {
        let json = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        compiler = compiler.metadata_json(json);
    }
    for (key, value) in args.meta {
        compiler = compiler.metadata_entry(key, value);
    }
    let bytes = compiler
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

/// The time now, which the compiler is told because it never reads the clock itself.
fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}
