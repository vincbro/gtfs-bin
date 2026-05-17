use std::{fs::File, io::Write, path::PathBuf, time::Instant};

use clap::Parser;
use gtfs_bin::compiler::Compiler;
use spinners::{Spinner, Spinners};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long, default_value = "output.gtfs")]
    output: PathBuf,
}

fn main() {
    let args = Args::parse();
    let now = Instant::now();
    let mut sp = Spinner::new(Spinners::Dots, "Compiling data".to_string());
    let bytes = Compiler::new(args.input)
        .compile()
        .expect("Failed to build");
    sp.stop_with_message(format!(
        "Loading and compiling gtfs took {:?}",
        now.elapsed()
    ));

    let now = Instant::now();
    let mut sp = Spinner::new(Spinners::Dots, "Writing output to disk".to_string());
    let mut file = File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .open(args.output)
        .expect("Failed to open file");
    file.write_all(&bytes)
        .expect("Failed to write binary to file");
    sp.stop_with_message(format!("Writing binary to file took {:?}", now.elapsed()));
}
