use std::{path::PathBuf, time::Instant};

use clap::Parser;
use gtfsbin::builder::Builder;

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
    let _bytes = Builder::new(args.input).build().expect("Failed to build");
    println!("Loading gtfs took {:?}", now.elapsed());
}
