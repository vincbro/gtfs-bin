use std::{fs::File, io::Write, path::PathBuf, time::Instant};

use clap::Parser;
use gtfs_bin::compiler::Compiler;

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
    let bytes = Compiler::new(args.input)
        .compile()
        .expect("Failed to build");
    println!("Loading and compiling gtfs took {:?}", now.elapsed());
    let mut file = File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .open(args.output)
        .expect("Failed to open file");
    file.write_all(&bytes)
        .expect("Failed to write binary to file");
}
