//! Opens a compiled file and reports what it holds and how long it took to open.
//!
//! Run with `cargo run --release --example inspect -- network.gtfs`.
//!
//! Add `near <latitude> <longitude> <meters>` to list the stops around a position, or
//! `search <text>` to look a place up by name.

use std::{env, fs::File, time::Instant};

use anyhow::Context;
use gtfs_bin_core::consumer::Consumer;
use memmap2::Mmap;

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    let path = args.next().context("usage: inspect <compiled file>")?;
    let file = File::open(&path).with_context(|| format!("failed to open {path}"))?;
    // SAFETY: the file is only read, and nothing here modifies it while it is mapped.
    let mmap = unsafe { Mmap::map(&file) }.with_context(|| format!("failed to map {path}"))?;

    let started = Instant::now();
    let consumer = Consumer::new(&mmap).with_context(|| format!("{path} is not a valid file"))?;
    println!("opened and validated in {:?}", started.elapsed());

    println!("{} bytes", mmap.len());
    println!("{} stops", consumer.stops().len());
    println!("{} routes", consumer.routes().len());
    println!("{} trips", consumer.trips().len());
    println!("{} stop times", consumer.stop_times().len());
    println!("{} trip patterns", consumer.trip_patterns().len());

    // A scan like a router's: every trip's stop times, summed so the loop isn't optimized away.
    let started = Instant::now();
    let mut seconds = 0_u64;
    for trip in consumer.trips() {
        for stop_time in consumer.stop_times_by_trip(trip.index) {
            seconds += u64::from(
                stop_time
                    .departure_time
                    .as_option()
                    .map_or(0, |time| time.0),
            );
        }
    }
    println!(
        "scanned every stop time in {:?} ({seconds})",
        started.elapsed()
    );
    let started = Instant::now();
    consumer
        .verify_checksum()
        .context("the checksum is wrong")?;
    println!("checksum verified in {:?}", started.elapsed());
    println!(
        "time zone {:?}, service days {:?}",
        consumer.timezone(),
        consumer.service_range()
    );

    match args.next().as_deref() {
        Some("near") => {
            let mut number = || -> anyhow::Result<f64> {
                args.next()
                    .context("near needs a latitude, a longitude and a radius in meters")?
                    .parse()
                    .context("not a number")
            };
            let (latitude, longitude, radius) = (number()?, number()?, number()?);
            let mut found = Vec::new();
            let started = Instant::now();
            consumer.stops_near(latitude, longitude, radius, &mut found);
            println!(
                "{} stops within {radius} m in {:?}",
                found.len(),
                started.elapsed()
            );
            for &(stop, meters) in found.iter().take(10) {
                let stop = consumer.stop(stop);
                let name = stop
                    .name
                    .as_option()
                    .map_or("", |name| consumer.string(name));
                println!("  {meters:6.0} m  {name}");
            }
        }
        Some("search") => {
            let query = args.collect::<Vec<_>>().join(" ");
            let mut found = Vec::new();
            let started = Instant::now();
            consumer.search_by_name(&query, &mut found);
            println!(
                "{} places match {query:?} in {:?}",
                found.len(),
                started.elapsed()
            );
            for &index in found.iter().take(10) {
                println!("  {}", consumer.string(consumer.search(index).name));
            }
        }
        _ => {}
    }
    Ok(())
}
