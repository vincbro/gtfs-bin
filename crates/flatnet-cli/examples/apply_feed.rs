//! Applies GTFS-realtime files to a compiled file and says what matched.
//!
//! Run with `cargo run --release --example apply_feed -- network.flatnet TripUpdates.pb`.

use std::{env, fs, fs::File, time::Instant};

use anyhow::Context;
use flatnet_core::{
    consumer::Consumer,
    rt::{self, Realtime, RealtimeBuilder},
};
use memmap2::Mmap;

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .context("usage: apply_feed <compiled file> <feed.pb>...")?;
    let file = File::open(&path).with_context(|| format!("failed to open {path}"))?;
    // SAFETY: the file is only read, and nothing here modifies it while it is mapped.
    let mmap = unsafe { Mmap::map(&file) }.with_context(|| format!("failed to map {path}"))?;
    let consumer = Consumer::new(&mmap).with_context(|| format!("{path} is not a valid file"))?;
    println!("timezone: {:?}", consumer.timezone());

    let builder = RealtimeBuilder::new(&consumer).with_preferred_language("sv");
    let mut realtime = Realtime::new(&consumer);
    for feed_path in args {
        let bytes = fs::read(&feed_path).with_context(|| format!("failed to read {feed_path}"))?;
        let message =
            rt::decode_feed(&bytes).with_context(|| format!("failed to decode {feed_path}"))?;
        println!(
            "{feed_path}: {} entities, timestamp {:?}",
            message.entity.len(),
            message.header.timestamp
        );
        let started = Instant::now();
        let report = builder.apply([message].into_iter(), &mut realtime);
        println!("  applied in {:?}: {report:#?}", started.elapsed());
    }

    let delayed = realtime
        .stop_time_departure_delays()
        .iter()
        .filter(|delay| delay.is_some())
        .count();
    println!("{delayed} stop times have a delay");
    println!(
        "{} vehicles",
        realtime.trip_vehicles().iter().flatten().count()
    );
    println!("{} alerts", realtime.alerts().len());
    println!("{} added trips", realtime.added_trips().len());
    Ok(())
}
